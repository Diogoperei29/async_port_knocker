# Async Port Knocker Scanner

[![Rust](https://img.shields.io/badge/Rust-%23000000.svg?e&logo=rust&logoColor=white)](#)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A small Rust CLI for sending TCP/UDP "knocks" to a host.  
Supports async concurrency, inter-knock delays, and custom UDP payloads.  
This was done as a fun project to improve my RUST skills :)

## Features

- TCP & UDP knocking  
- Configurable timeout per knock (`--timeout`)  
- Inter-knock delay with random jitter (`--delay`)  
- Max concurrency (`--concurrency`)  
- Hex-encoded UDP payloads (`--payload`)  
- Retries (`--retries`) with backoff (`--backoff`)  
- IPv4 & IPv6 support  
- Randomized UDP source port for stealth/fingerprint evasion  
- UDP response capture (ICMP & UDP replies)  
- Graceful shutdown on Ctrl-C  
- DNS pre-resolution and reuse for all knocks  
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

- Propper logging (e.g. `tracing`)  
- Read parameters from files
- ?Publish to crates.io

## Security Considerations

- Authorization: always obtain explicit permission before scanning or knocking on remote hosts.
- Payload Validity: many firewalls silently drop malformed or zero-length UDP datagrams—successful `send_to` does not guarantee delivery.
- ICMP Feedback: lack of ICMP unreachable replies does not imply a port is open; UDP scans remain inherently blind.
- Fingerprinting & Stealth: use randomized delays (jitter), exponential backoff, and varied payload sizes to evade IDS/IPS signature detection (implemented).
- Source Port Randomization: binding to random local ports per knock reduces predictability and fingerprintability (implemented).
- Dual-Stack Targets: ensure both IPv4 and IPv6 addresses are scanned in modern networks.

