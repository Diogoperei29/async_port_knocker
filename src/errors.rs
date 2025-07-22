use thiserror::Error;

/// Top-level application errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error("network I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no DNS records found for target")]
    NoDns,
}
