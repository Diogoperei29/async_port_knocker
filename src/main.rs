use async_port_knocker::{cli::Cli, run};
use clap::Parser;

#[tokio::main]
async fn main() {
    // Parse command-line arguments using the definition from the library.
    let cli = Cli::parse();

    // Execute the main application logic from the library.
    // If an error occurs, print it to stderr and exit with a non-zero code.
    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
