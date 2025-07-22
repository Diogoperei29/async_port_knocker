use clap::{Parser, ValueEnum};
use futures::StreamExt;
use hex;
use tokio::{
    net::{TcpStream, UdpSocket},
    time::{sleep, timeout, Duration},
};

/// Async Port Knocker Scanner
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Target host (IP or hostname)
    #[arg(long)]
    host: String,

    /// Protocol to use for knocks
    #[arg(short, long, value_enum, default_value_t = Protocol::Tcp)]
    protocol: Protocol,

    /// Comma-separated port sequence (e.g. "7000,8000,9000")
    #[arg(short, long)]
    sequence: String,

    /// Timeout per knock in milliseconds
    #[arg(short, long, default_value_t = 500)]
    timeout: u64,

    /// Inter-knock delay in milliseconds
    #[arg(long, default_value_t = 0)]
    delay: u64,

    /// Max concurrent knocks
    #[arg(long, default_value_t = 1)]
    concurrency: usize,

    /// UDP payload as hex (e.g. "deadbeef")
    #[arg(long, default_value_t = String::new())]
    payload: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Protocol {
    Tcp,
    Udp,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Parse ports
    let ports = parse_ports(&cli.sequence).map_err(|e| {
        eprintln!("Port parse error: {}", e);
        e
    })?;

    // Decode payload once
    let payload_bytes = if !cli.payload.is_empty() {
        Some(hex::decode(&cli.payload).map_err(|e| {
            eprintln!("Invalid hex payload: {}", e);
            e
        })?)
    } else {
        None
    };

    // Build a stream of knock futures
    let knock_futs = ports.into_iter().map(|port| {
        let host = cli.host.clone();
        let proto = cli.protocol;
        let to_ms = cli.timeout;
        let delay_ms = cli.delay;
        let payload = payload_bytes.clone();
        async move {
            if delay_ms > 0 {
                sleep(Duration::from_millis(delay_ms)).await;
            }
            match proto {
                Protocol::Tcp => {
                    knock_tcp(&host, port, to_ms).await;
                }
                Protocol::Udp => {
                    knock_udp(&host, port, to_ms, payload.clone()).await;
                }
            }
        }
    });

    // Drive them with limited concurrency
    futures::stream::iter(knock_futs)
        .buffer_unordered(cli.concurrency)
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}

async fn knock_tcp(host: &str, port: u16, to_ms: u64) {
    let addr = (host, port);
    match timeout(Duration::from_millis(to_ms), TcpStream::connect(addr)).await {
        Ok(Ok(s)) => {
            println!("TCP {}:{} OK", host, port);
            drop(s);
        }
        Ok(Err(e)) => eprintln!("TCP {}:{} ERR {}", host, port, e),
        Err(_) => eprintln!("TCP {}:{} TIMEOUT", host, port),
    }
}

async fn knock_udp(host: &str, port: u16, to_ms: u64, payload: Option<Vec<u8>>) {
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("UDP bind error: {}", e);
            return;
        }
    };
    let data = payload.unwrap_or_default();
    match timeout(
        Duration::from_millis(to_ms),
        socket.send_to(&data, (host, port)),
    )
    .await
    {
        Ok(Ok(n)) => {
            println!("UDP {}:{} sent {} bytes", host, port, n);
        }
        Ok(Err(e)) => eprintln!("UDP {}:{} ERR {}", host, port, e),
        Err(_) => eprintln!("UDP {}:{} TIMEOUT", host, port),
    }
}

/// Parse a comma-separated list of u16 ports
fn parse_ports(s: &str) -> Result<Vec<u16>, String> {
    let mut v = Vec::new();
    for part in s.split(',') {
        let p = part
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("'{}' is not a valid port", part))?;
        v.push(p);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::parse_ports;

    #[test]
    fn valid_sequence() {
        assert_eq!(parse_ports("80,443").unwrap(), vec![80, 443]);
    }

    #[test]
    fn invalid_sequence() {
        assert!(parse_ports("foo,123").is_err());
    }
}
