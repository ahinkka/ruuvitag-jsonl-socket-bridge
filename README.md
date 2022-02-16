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

## References

- [Documentation for env\_logger](https://docs.rs/env_logger/latest/env_logger/)
