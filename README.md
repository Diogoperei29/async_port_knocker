# Async Port Knocker Scanner

A small Rust CLI for sending TCP/UDP "knocks" to a host.  
Supports async concurrency, inter-knock delays, and custom UDP payloads.  
This was done as a fun project to learn RUST :)

## Features

- TCP & UDP knocking  
- Configurable timeout per knock (`--timeout`)  
- Inter-knock delay (`--delay`)  
- Max concurrency (`--concurrency`)  
- Hex-encoded UDP payloads (`--payload`)  
- Retries (`--retries`) with backoff (`--backoff`)  
- IPv4 & IPv6 support  
- Unit tests for port parsing  
- CI: `cargo fmt`, `clippy`, `test`

## Requirements

- Rust stable (1.65+)
- `cargo` (comes with `rustup`)

## Installation

```bash
git clone https://github.com/diogoperei29/async_port_knocker.git
cd async_port_knocker
cargo build --release
```

## Usage 

#### Basic TCP knock:
```bash
cargo run --release -- \
  --host scanme.nmap.org \
  --protocol tcp \
  --sequence 22,81 \
  --timeout 300
```

#### Advanced usage:
```bash
cargo run --release -- \
  --host example.com \
  --protocol udp \
  --sequence 1000,2000,3000 \
  --timeout 200 \
  --delay 50 \
  --concurrency 2 \
  --payload deadbeef \
  --retries 3 \
  --backoff 150
```

#### IPv6 example:
```bash
cargo run --release -- \
  --host 2606:4700:4700::1111 \
  --protocol udp \
  --sequence 53 \
  --retries 2
```

## Next Steps

- Better logging (e.g. `tracing`)  
- ?Publish to crates.io

## License

MIT
