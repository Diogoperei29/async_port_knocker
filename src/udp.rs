use rand::{rngs::ThreadRng, RngCore};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    net::UdpSocket,
    time::{sleep, timeout, Duration},
};

/// Perform a UDP knock with retries, random source port, and optional reply.
pub(crate) async fn knock_udp(
    host: Arc<String>,
    port: u16,
    to_ms: u64,
    retries: usize,
    backoff: u64,
    ips: Arc<Vec<SocketAddr>>,
    payload: Option<Arc<Vec<u8>>>,
) {
    // Copy first resolved address (SocketAddr is Copy), set port
    let mut target = ips.first().copied().unwrap();
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
            return;
        }
    };

    // Convert Option<Arc<Vec<u8>>> into a byte slice
    let data: &[u8] = match &payload {
        Some(buf) => buf.as_slice(),
        None => &[],
    };
    let mut buf = vec![0u8; 1500];

    for attempt in 1..=retries {
        // Send datagram with timeout
        match timeout(Duration::from_millis(to_ms), socket.send_to(data, target)).await {
            Ok(Ok(_)) => {
                // Try to catch any ICMP or UDP reply
                match timeout(Duration::from_millis(to_ms), socket.recv_from(&mut buf)).await {
                    Ok(Ok((nrecv, src))) => {
                        println!("UDP {host}:{port} received {nrecv} bytes from {src}");
                        return;
                    }
                    Ok(Err(e)) => eprintln!("UDP {host}:{port} recv ERR {e}"),
                    Err(_) => eprintln!("UDP {host}:{port} no response (recv timeout)"),
                }
            }
            Ok(Err(e)) => eprintln!("UDP {host}:{port} send ERR {e}"),
            Err(_) => eprintln!("UDP {host}:{port} send TIMEOUT"),
        }
        if attempt < retries {
            sleep(Duration::from_millis(backoff)).await;
        }
    }
}
