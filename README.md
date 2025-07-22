# Async Port Knocker Scanner

A small Rust CLI for sending TCP/UDP "knocks" to a host.  
Supports async concurrency, inter-knock delays, and custom UDP payloads.

## Features

- TCP & UDP knocking
- Configurable timeout per knock
- Inter-knock delay (`--delay`)
- Max concurrency (`--concurrency`)
- Hex-encoded UDP payloads (`--payload`)
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

#### basic TCP sequence
```bash
cargo run --release -- \
  --host example.com \
  --protocol tcp \
  --sequence 7000,8000,9000
```
#### with delay & concurrency
```bash
cargo run --release -- \
  --host 192.0.2.1 \
  --protocol udp \
  --sequence 1000,2000,3000 \
  --timeout 200 \
  --delay 50 \
  --concurrency 2 \
  --payload deadbeef
```