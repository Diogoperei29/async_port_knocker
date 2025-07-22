use clap::{Parser, ValueEnum};
use std::sync::Arc;

/// Async TCP/UDP Port Knocker Scanner CLI
#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// Target host (IP or hostname) to knock on
    #[arg(short = 'H', long)]
    pub host: String,

    /// Protocol to use for knocks: tcp or udp
    #[arg(short, long, value_enum, default_value_t = Protocol::Tcp)]
    pub protocol: Protocol,

    /// Comma-separated port sequence (e.g. "7000,8000,9000")
    #[arg(short, long, value_parser = parse_port, value_delimiter = ',')]
    pub sequence: Vec<u16>,

    /// Timeout per knock in milliseconds
    #[arg(short, long, default_value_t = 500)]
    pub timeout: u64,

    /// Inter-knock base delay in milliseconds
    #[arg(long, default_value_t = 0)]
    pub delay: u64,

    /// Max concurrent knocks
    #[arg(long, default_value_t = 1)]
    pub concurrency: usize,

    /// Optional UDP payload as hex (e.g. "deadbeef")
    #[arg(long, value_parser = parse_hex_payload)]
    pub payload: Option<Arc<Vec<u8>>>,

    /// Number of retries per knock
    #[arg(short = 'r', long, default_value_t = 1)]
    pub retries: usize,

    /// Backoff between retries in milliseconds
    #[arg(short = 'b', long, default_value_t = 100)]
    pub backoff: u64,
}

/// Supported knock protocols
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Protocol {
    Tcp,
    Udp,
}

/// Parse a comma‐free single port argument into u16.
pub fn parse_port(s: &str) -> Result<u16, String> {
    s.parse::<u16>()
        .map_err(|_| format!("'{s}' is not a valid port"))
}

/// Decode a hex payload string into an Arc‐wrapped Vec<u8>.
pub fn parse_hex_payload(s: &str) -> Result<Arc<Vec<u8>>, String> {
    hex::decode(s)
        .map(Arc::new)
        .map_err(|e| format!("invalid hex payload: {e}"))
}

#[cfg(test)]
mod tests {
    use super::parse_port;

    #[test]
    fn valid_port() {
        assert_eq!(parse_port("80").unwrap(), 80);
    }

    #[test]
    fn invalid_port() {
        assert!(parse_port("foo").is_err());
    }
}
