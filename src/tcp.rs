use std::sync::Arc;
use tokio::{
    net::TcpStream,
    time::{sleep, timeout, Duration},
};

/// Perform a TCP knock with per-attempt logging, retries, timeouts and backoff.
pub(crate) async fn knock_tcp(
    host: Arc<String>,
    port: u16,
    to_ms: u64,
    retries: usize,
    backoff: u64,
) {
    for attempt in 1..=retries {
        match timeout(
            Duration::from_millis(to_ms),
            TcpStream::connect((host.as_str(), port)),
        )
        .await
        {
            // Connected successfully
            Ok(Ok(_stream)) => {
                println!("TCP {host}:{port} OK");
                return;
            }
            // Got an immediate I/O error
            Ok(Err(e)) => {
                eprintln!("TCP {host}:{port} ERR {e}");
            }
            // Timed out before connect completed
            Err(_) => {
                eprintln!("TCP {host}:{port} TIMEOUT");
            }
        }

        // If we're going to retry, wait the backoff interval
        if attempt < retries {
            sleep(Duration::from_millis(backoff)).await;
        }
    }
}
