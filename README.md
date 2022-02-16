# ruuvitag-jsonl-socket-bridge

Bridge Bluetooth LE manufacturer data advertisements into newline-delimited
JSON over socket. This decouples the Bluetooth specific parts from the actual
uses. I.e. it's possible to build both streaming and persistence and not have
to worry about Bluetooth specifics.

## How to develop

To see full debug logging, run with
```
RUST_LOG=trace cargo run
```

To build with just 1 thread concurrently, run
```
cargo build --jobs=1
```

## Installation

As root on the system where you are running this (typically a Raspberry Pi dedicated to this purpose):
```
cp target/release/ruuvitag-jsonl-socket-bridge /usr/bin/
chown root:root /usr/bin/ruuvitag-jsonl-socket-bridge
useradd -s /usr/sbin/nologin -r -M ruuvi-bridge
cp ruuvitag-jsonl-socket-bridge.service /etc/systemd/system/
chmod 664 /etc/systemd/system/ruuvitag-jsonl-socket-bridge.service
systemctl daemon-reload
systemctl enable ruuvitag-jsonl-socket-bridge
```

## References

- [Documentation for
  env\_logger](https://docs.rs/env_logger/latest/env_logger/)
