//! BlindHop Proxy — Local WebSocket proxy that routes smoldot traffic
//! through the Nym mixnet for metadata privacy.
//!
//! # Architecture
//!
//! ```text
//! Smoldot (Browser) --WS--> blindhop-proxy --Nym--> Nym Mixnet --> blindhop-exit --> Substrate Full Node
//! ```
//!
//! # Privacy Modes
//!
//! - **None**: Direct WebSocket passthrough (no privacy, lowest latency)
//! - **Fast**: 2-hop Nym dVPN mode (IP hidden, ~200-500ms overhead)
//! - **Full**: 5-hop Nym mixnet + cover traffic (metadata private, ~1-3s overhead)

mod bridge;
mod mode;
mod nym_transport;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::net::TcpListener;

use blindhop_common::config::{ExitBackendType, PrivacyMode};

#[derive(Parser)]
#[command(
    name = "blindhop-proxy",
    about = "BlindHop local WebSocket proxy — routes smoldot traffic through the Nym mixnet",
    version
)]
struct Cli {
    /// Listen address for incoming smoldot connections.
    #[arg(short, long, default_value = "127.0.0.1:9500")]
    listen: SocketAddr,

    /// Target Substrate full node WebSocket URL.
    #[arg(long, default_value = "wss://sys.turboflakes.io/asset-hub-paseo")]
    target: String,

    /// Privacy mode: none, fast, or full.
    #[arg(long, default_value = "full")]
    privacy_mode: String,

    /// Exit backend type: service-provider. (socks5 is not implemented yet
    /// and is refused at startup.)
    #[arg(long, default_value = "service-provider")]
    exit_backend: String,

    /// Nym address of the BlindHop exit service.
    #[arg(long)]
    exit_address: Option<String>,

    /// Nym gateway to connect through (auto-selected if not specified).
    #[arg(long)]
    nym_gateway: Option<String>,

    /// Browser origin allowed to connect, e.g. https://demo.blindhop.wtf.
    /// Repeat for several. Without it, only clients that send no Origin
    /// header (smoldot, CLI tools) can connect; any web page is refused.
    #[arg(long = "allowed-origin", value_name = "ORIGIN")]
    allowed_origins: Vec<String>,
}

impl Cli {
    fn privacy_mode(&self) -> PrivacyMode {
        match self.privacy_mode.to_lowercase().as_str() {
            "none" => PrivacyMode::None,
            "fast" => PrivacyMode::Fast,
            "full" => PrivacyMode::Full,
            _ => {
                tracing::warn!(
                    "Unknown privacy mode '{}', defaulting to Full",
                    self.privacy_mode
                );
                PrivacyMode::Full
            }
        }
    }

    fn exit_backend(&self) -> ExitBackendType {
        match self.exit_backend.to_lowercase().as_str() {
            "service-provider" | "sp" => ExitBackendType::ServiceProvider,
            "socks5" | "socks" => ExitBackendType::Socks5,
            _ => {
                tracing::warn!(
                    "Unknown exit backend '{}', defaulting to ServiceProvider",
                    self.exit_backend
                );
                ExitBackendType::ServiceProvider
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    let privacy_mode = cli.privacy_mode();
    let exit_backend = cli.exit_backend();
    if exit_backend == ExitBackendType::Socks5 {
        // Fail rather than silently use a different backend than requested.
        anyhow::bail!("The socks5 exit backend is not implemented yet; use service-provider");
    }

    let config = blindhop_common::config::BlindHopConfig {
        listen_addr: cli.listen,
        target_rpc: cli.target.clone(),
        privacy_mode,
        exit_backend,
        exit_address: cli.exit_address.clone(),
        nym_gateway: cli.nym_gateway.clone(),
        allowed_origins: cli.allowed_origins.clone(),
        ..Default::default()
    };

    tracing::info!("Starting BlindHop Proxy (Nym Mixnet)");
    tracing::info!("  Listen:       {}", config.listen_addr);
    tracing::info!("  Target:       {}", config.target_rpc);
    tracing::info!(
        "  Privacy:      {} {}",
        privacy_mode.indicator(),
        privacy_mode
    );
    tracing::info!("  Exit backend: {}", exit_backend);
    if let Some(ref addr) = config.exit_address {
        tracing::info!("  Exit address: {}", addr);
    }
    if config.allowed_origins.is_empty() {
        tracing::info!("  Origins:      none (browser pages are refused)");
    } else {
        tracing::info!("  Origins:      {}", config.allowed_origins.join(", "));
    }

    let listener = TcpListener::bind(config.listen_addr)
        .await
        .with_context(|| format!("Failed to bind to {}", config.listen_addr))?;

    tracing::info!("Proxy listening on ws://{}", config.listen_addr);

    // Start accepting connections
    bridge::run_proxy(listener, config).await
}
