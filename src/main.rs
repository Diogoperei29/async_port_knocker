use clap::{Parser, ValueEnum};
use futures::StreamExt;
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
    let ports = match parse_ports(&cli.sequence) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Port parse error: {e}");
            return Ok(());
        }
    };

    // Decode UDP payload once, if any
    let payload_bytes = if cli.payload.is_empty() {
        None
    } else {
        let data =
            hex::decode(&cli.payload).inspect_err(|e| eprintln!("Invalid hex payload: {e}"))?;
        Some(data)
    };

    // Build and run knock futures with limited concurrency
    let stream = futures::stream::iter(ports.into_iter().map(|port| {
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
                Protocol::Tcp => knock_tcp(&host, port, to_ms).await,
                Protocol::Udp => knock_udp(&host, port, to_ms, payload.clone()).await,
            }
        }
    }))
    .buffer_unordered(cli.concurrency);

    stream.for_each(|_| futures::future::ready(())).await;
    Ok(())
}

async fn knock_tcp(host: &str, port: u16, to_ms: u64) {
    match timeout(
        Duration::from_millis(to_ms),
        TcpStream::connect((host, port)),
    )
    .await
    {
        Ok(Ok(_s)) => println!("TCP {host}:{port} OK"),
        Ok(Err(e)) => eprintln!("TCP {host}:{port} ERR {e}"),
        Err(_) => eprintln!("TCP {host}:{port} TIMEOUT"),
    }
}

async fn knock_udp(host: &str, port: u16, to_ms: u64, payload: Option<Vec<u8>>) {
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("UDP bind error: {e}");
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
        Ok(Ok(n)) => println!("UDP {host}:{port} sent {n} bytes"),
        Ok(Err(e)) => eprintln!("UDP {host}:{port} ERR {e}"),
        Err(_) => eprintln!("UDP {host}:{port} TIMEOUT"),
    }
}

/// Parse a comma-separated list of u16 ports
fn parse_ports(s: &str) -> Result<Vec<u16>, String> {
    let mut v = Vec::new();
    for part in s.split(',') {
        let p = part
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("'{part}' is not a valid port"))?;
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
