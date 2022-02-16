#[macro_use]
extern crate log;

extern crate btleplug;
extern crate ruuvi_sensor_protocol;

use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::Manager;
use futures::stream::StreamExt;
use std::error::Error;

use ruuvi_sensor_protocol::SensorValues;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting up...");

    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await?;

    debug!("Listing adapters...");
    for adapter in &adapters {
	debug!("{}", adapter.adapter_info().await?);
    }

    let adapter = adapters.get(0).unwrap();
    info!("Using adapter: {}", adapter.adapter_info().await?);

    let mut events = adapter.events().await?;
    let start_result = adapter.start_scan(ScanFilter::default()).await?;
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
                trace!(
		    "ManufacturerDataAdvertisement: {:?}, {:?}",
		    id, manufacturer_data
                );
		for (manufacturer_id, bytes) in &manufacturer_data {
		    let parsed = SensorValues::from_manufacturer_specific_data(manufacturer_id.clone(), bytes);
		    trace!("parsed: {:?}", parsed);
		}
	    }
	    // TODO: some kind of "exit if we haven't received any valid events in a while" functionality
	    _ => {}
        }
    }

    let stop_result = adapter.stop_scan().await?;
    info!("Scan stopped: {:?}", stop_result);

    Ok(())
}
