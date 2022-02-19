#[macro_use]
extern crate log;
extern crate btleplug;
extern crate ruuvi_sensor_protocol;

use std::error::Error;

use structopt::StructOpt;

use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::{Manager};
use futures::stream::StreamExt;
use tokio::sync::broadcast;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;

use ruuvi_sensor_protocol::{AccelerationVector, SensorValues};
use crate::ruuvi_sensor_protocol::Acceleration;
use crate::ruuvi_sensor_protocol::BatteryPotential;
use crate::ruuvi_sensor_protocol::Humidity;
use crate::ruuvi_sensor_protocol::MacAddress;
use crate::ruuvi_sensor_protocol::MeasurementSequenceNumber;
use crate::ruuvi_sensor_protocol::MovementCounter;
use crate::ruuvi_sensor_protocol::Pressure;
use crate::ruuvi_sensor_protocol::Temperature;
use crate::ruuvi_sensor_protocol::TransmitterPower;
use serde_json::json;

async fn bt_event_scan(
    tx: broadcast::Sender<SensorValues>
) -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await.unwrap();

    let adapters = manager.adapters().await?;
    debug!("Listing adapters...");
    for adapter in &adapters {
	debug!("{}", adapter.adapter_info().await?);
    }

    let adapter = adapters.get(0).unwrap();
    info!("Using adapter: {}", adapter.adapter_info().await?);

    let mut events = adapter.events().await?;
    let start_result = adapter.start_scan(ScanFilter::default()).await;
    info!("Scan started: {:?}", start_result);

    while let Some(event) = events.next().await {
        match event {
	    // https://docs.rs/btleplug/0.9.0/btleplug/api/enum.CentralEvent.html
	    // TODO: add back with seen already filtering
	    // CentralEvent::DeviceDiscovered(id) => {
            //     eprintln!("DeviceDiscovered: {:?}", id);
	    // }
	    CentralEvent::ManufacturerDataAdvertisement {
                id,
                manufacturer_data,
	    } => {
                debug!(
		    "ManufacturerDataAdvertisement: {:?}, {:?}",
		    id, manufacturer_data
                );
		for (manufacturer_id, bytes) in &manufacturer_data {
		    let parsed = SensorValues::from_manufacturer_specific_data(manufacturer_id.clone(), bytes);
		    trace!("parsed: {:?}", parsed);
		    match parsed {
			Ok(sv) => {
			    let recipients = tx.send(sv);
			    trace!("Message was sent to {:?}", recipients)
			},
			Err(e) => {
			    match e {
				ruuvi_sensor_protocol::ParseError::UnknownManufacturerId(_id) =>
				    debug!("Got unknown manufacturer id: {:?}", e),
				_ => error!("Failed to parse manufacturer data advertisement: {:?}", e)
			    }
			}
		    }
		}
	    }
	    // TODO: some kind of "exit if we haven't received any valid events in a while" functionality
	    _ => {}
        }
    }

    let stop_result = adapter.stop_scan().await;
    info!("Scan stopped: {:?}", stop_result);

    Ok(())
}

async fn handle_socket(
    mut socket: TcpStream,
    mut receiver: broadcast::Receiver<SensorValues>
) {
    info!("New socket connection: {:?}", socket);
    loop {
	let sv = receiver.recv().await.unwrap();
	trace!("Socket RX {:?}", sv);

	let value = json!({
	    "acceleration_vector_as_milli_g": sv.acceleration_vector_as_milli_g().map(|av| {
		match av {
		    AccelerationVector(a, b, c) => Some(vec!(a, b, c)),
		}
	    }),
	    "battery_potential_as_millivolts": sv.battery_potential_as_millivolts(),
	    "humidity_as_ppm": sv.humidity_as_ppm(),
	    "mac_address": sv.mac_address(),
	    "measurement_sequence_number": sv.measurement_sequence_number(),
	    "movement_counter": sv.movement_counter(),
	    "pressure_as_pascals": sv.pressure_as_pascals(),
	    "temperature_as_millikelvins": sv.temperature_as_millikelvins(),
	    "temperature_as_millicelsius": sv.temperature_as_millicelsius(),
	    "tx_power_as_dbm": sv.tx_power_as_dbm()
	});

	let s = value.to_string();
	let json_bytes = s.as_bytes();
	let newline_bytes = b"\r\n";

	let json_write_res = socket.write_all(&json_bytes).await;
	let newline_write_res = socket.write_all(newline_bytes).await;
	let flush_res = socket.flush().await;
	match json_write_res.and(newline_write_res).and(flush_res) {
	    Ok(v) => trace!("Socket write and flush: {:?}", v),
	    Err(e) => {
		match e.kind() {
		    std::io::ErrorKind::BrokenPipe => {
			info!("Closing socket: {:?}", e);
			let _ = socket.shutdown().await;
			break
		    },
		    _ => warn!("Failed to write or flush socket: {:?}", e)
		}
	    }
	}
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "ruuvi-jsonl-socket-bridge",
    about = "Bridge Ruuvi observations to a socket",
    no_version
)]
struct Opt {
    /// Host/IP address to listen on
    #[structopt(short, long, default_value = "localhost")]
    hostname: String,

    /// Port
    // we don't want to name it "speed", need to look smart
    #[structopt(short, long, default_value = "22222")]
    port: i16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
	.format_timestamp(None)
	.init();

    let opt = Opt::from_args();
    info!("CLI opts: {:?}", opt);
    info!("Starting up...");

    let (tx, mut _rx) = broadcast::channel::<SensorValues>(32);
    let tx2 = tx.clone();

    // Listener task for debugging:
    // tokio::spawn(async move {
    //     trace!("RX started");
    // 	loop {
    // 	    let sv = rx.recv().await;
    // 	    trace!("RX {:?}", sv);
    // 	}
    // });

    let _bt_task = tokio::spawn(async move {
	let _ = bt_event_scan(tx).await;
    });

    let mut bind_addr = opt.hostname.to_owned();
    bind_addr.push_str(&":");
    bind_addr.push_str(&opt.port.to_string());

    debug!("Starting socket listener at {:?}", bind_addr);
    let listener = TcpListener::bind(bind_addr).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
	let receiver = tx2.subscribe();
	tokio::spawn(async move {
	    handle_socket(socket, receiver).await;
	});
    }
}
