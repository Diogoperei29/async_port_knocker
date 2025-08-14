use crate::{retry::retry_with_backoff, AppError};
use rand::{rngs::ThreadRng, RngCore};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

/// Perform a UDP knock with retries, random source port, and optional reply.
pub(crate) async fn knock_udp(
    host: Arc<String>,
    port: u16,
    to_ms: u64,
    retries: usize,
    backoff: u64,
    ips: Arc<Vec<SocketAddr>>,
    payload: Option<Arc<Vec<u8>>>,
) -> Result<(), AppError> {
    // Copy first resolved address (SocketAddr is Copy), set port
    let mut target = match ips.first().copied() {
        Some(addr) => addr,
        None => return Err(AppError::NoDns),
    };
    target.set_port(port);

    // Pick a random local ephemeral port
    let mut rng: ThreadRng = ThreadRng::default();
    let range = (61000 - 32768) as u32;
    let offset = rng.next_u32() % range;
    let local_port = 32768 + offset as u16;

    // Bind UDP socket on that port
    let bind_addr = if target.is_ipv6() {
        format!("[::]:{local_port}")
    } else {
        format!("0.0.0.0:{local_port}")
    };
    let socket = match UdpSocket::bind(&bind_addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("UDP {host}:{port} bind ERR {e}");
            return Ok(()); // keep same behavior for bind errors
        }
    };

    // Convert Option<Arc<Vec<u8>>> into a byte slice
    let data: &[u8] = match &payload {
        Some(buf) => buf.as_slice(),
        None => &[],
    };
    let buf = vec![0u8; 1500];

    retry_with_backoff(
        retries,
        to_ms,
        backoff,
        |attempt| {
            let socket = &socket;
            let mut buf = buf.clone();
            let host = host.clone();
            async move {
                // Send datagram
                match socket.send_to(data, target).await {
                    Ok(_) => {
                        // Try to catch any ICMP or UDP reply
                        match socket.recv_from(&mut buf).await {
                            Ok((nrecv, src)) => {
                                println!("UDP {host}:{port} received {nrecv} bytes from {src}");
                                Ok::<bool, AppError>(true) // stop retrying
                            }
                            Err(e) => {
                                eprintln!("UDP {host}:{port} recv ERR {e} (attempt {attempt})");
                                Ok::<bool, AppError>(false) // retry
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("UDP {host}:{port} send ERR {e} (attempt {attempt})");
                        Ok::<bool, AppError>(false) // retry
                    }
                }
            }
        },
        |attempt| {
            eprintln!("UDP {host}:{port} no response (recv timeout) (attempt {attempt})");
        },
    )
    .await?;

    Ok(())
}
