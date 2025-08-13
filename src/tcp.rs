use crate::retry::retry_with_backoff;
use std::sync::Arc;
use tokio::net::TcpStream;

/// Perform a TCP knock with per-attempt logging, retries, timeouts and backoff.
pub(crate) async fn knock_tcp(
    host: Arc<String>,
    port: u16,
    to_ms: u64,
    retries: usize,
    backoff: u64,
) {
    let host_for_timeout = host.clone();
    let _ = retry_with_backoff(
        retries,
        to_ms,
        backoff,
        |attempt| {
            let host = host.clone();
            async move {
                match TcpStream::connect((host.as_str(), port)).await {
                    // Connected successfully
                    Ok(_stream) => {
                        println!("TCP {host}:{port} OK");
                        Ok::<bool, ()>(true) // stop retrying
                    }
                    // Got an immediate I/O error
                    Err(e) => {
                        eprintln!("TCP {host}:{port} ERR {e} (attempt {attempt})");
                        Ok::<bool, ()>(false) // retry
                    }
                }
            }
        },
        |attempt| {
            eprintln!(
                "TCP {}:{} TIMEOUT (attempt {attempt})",
                host_for_timeout, port
            );
        },
    )
    .await;
}
