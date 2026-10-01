//! BlindHop Exit Service — Nym service provider that receives traffic
//! from the Nym mixnet and forwards it to Substrate full nodes.
//!
//! # Architecture
//!
//! ```text
//! Nym Mixnet --> blindhop-exit (Nym SP) --> Substrate Full Node (WS JSON-RPC)
//! ```
//!
//! The exit service:
//! 1. Registers as a Nym Service Provider
//! 2. Listens for incoming mixnet messages
//! 3. Deserializes JSON-RPC requests from the MixnetMessage envelope
//! 4. Forwards them to the target Substrate full node via WebSocket
//! 5. Wraps the response in a MixnetMessage and sends back through Nym

mod backend;
mod limiter;
mod policy;
mod service;
mod substrate_rpc;

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "blindhop-exit",
    about = "BlindHop exit service — Nym service provider for Substrate RPC forwarding",
    version
)]
struct Cli {
    /// Target Substrate full node WebSocket URL.
    #[arg(long, default_value = "wss://sys.turboflakes.io/asset-hub-paseo")]
    target_rpc: String,

    /// Directory for the exit's Nym keys and state. Keeping it across
    /// restarts keeps the exit's Nym address stable.
    #[arg(long, default_value = ".blindhop-exit")]
    data_dir: PathBuf,

    /// Log level.
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| cli.log_level.clone().into()),
        )
        // Colour codes only for a terminal; under systemd they end up in
        // the journal as escape sequences.
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    tracing::info!("Starting BlindHop Exit Service");
    tracing::info!("  Target RPC: {}", cli.target_rpc);
    tracing::info!("  Data dir:   {}", cli.data_dir.display());

    // Start the Nym service provider
    service::run_exit_service(&cli.target_rpc, &cli.data_dir).await
}
