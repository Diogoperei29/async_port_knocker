// Declare all the modules that make up this library.
pub mod cli;
pub mod errors;
pub mod retry;
pub mod tcp;
pub mod udp;

// Re-export the main run function and the Cli struct for the binary to use.
pub use cli::Cli;
pub use errors::AppError;
pub use retry::retry_with_backoff;

use crate::{tcp::knock_tcp, udp::knock_udp};
use futures::StreamExt;
use std::sync::Arc;
use tokio::{net::lookup_host, signal};

/// The main application logic.
/// This function is called by the binary's main function.
pub async fn run(cli: Cli) -> Result<(), AppError> {
    // Wrap host in Arc so tasks can share it cheaply
    let host = Arc::new(cli.host);

    // Pre-resolve DNS once
    let addrs = lookup_host((host.as_str(), 0)).await?.collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err(AppError::NoDns);
    }
    let ips = Arc::new(addrs);

    // Cloneable reference to optional UDP payload
    let payload = cli.payload.clone();

    // Build a future-per-port knock
    let knocks = cli.sequence.into_iter().map(|port| {
        let host = Arc::clone(&host);
        let ips = Arc::clone(&ips);
        let payload = payload.clone();
        let proto = cli.protocol;
        let to_ms = cli.timeout;
        let delay_ms = cli.delay;
        let retries = cli.retries;
        let backoff = cli.backoff;

        async move {
            // Inter-knock delay + random jitter
            if delay_ms > 0 {
                use rand::{rngs::ThreadRng, RngCore};
                use tokio::time::{sleep, Duration};
                let mut rng: ThreadRng = ThreadRng::default();
                let jitter = rng.next_u64() % (delay_ms + 1);
                sleep(Duration::from_millis(delay_ms + jitter)).await;
            }

            // Dispatch to TCP or UDP knock
            match proto {
                cli::Protocol::Tcp => {
                    knock_tcp(host.clone(), port, to_ms, retries, backoff).await;
                }
                cli::Protocol::Udp => {
                    if let Err(e) = knock_udp(
                        host.clone(),
                        port,
                        to_ms,
                        retries,
                        backoff,
                        ips.clone(),
                        payload.clone(),
                    )
                    .await
                    {
                        eprintln!("UDP knock error: {e}");
                    }
                }
            }
        }
    });

    // Run knocks with bounded concurrency, abort on Ctrl-C
    tokio::select! {
       _ = futures::stream::iter(knocks)
          .buffered(cli.concurrency)
          .for_each(|_| async {})
       => {}
       _ = signal::ctrl_c() => {
          eprintln!("Received Ctrl-C, aborting port knocks");
       }
    }

    Ok(())
}
