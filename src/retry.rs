use std::future::Future;
use tokio::time::{sleep, timeout, Duration};

/// generic async retry helper with timeout and backoff.
pub async fn retry_with_backoff<F, Fut, E, TCB>(
    retries: usize,
    timeout_ms: u64,
    backoff_ms: u64,
    mut operation: F,
    mut on_timeout: TCB,
) -> Result<(), E>
where
    F: FnMut(usize) -> Fut,
    Fut: Future<Output = Result<bool, E>>,
    TCB: FnMut(usize),
{
    for attempt in 1..=retries {
        match timeout(Duration::from_millis(timeout_ms), operation(attempt)).await {
            Ok(Ok(done)) => {
                if done {
                    return Ok(());
                }
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Timed out before operation completed
                on_timeout(attempt);
            }
        }

        // If we're going to retry, wait the backoff interval
        if attempt < retries {
            sleep(Duration::from_millis(backoff_ms)).await;
        }
    }
    Ok(())
}
