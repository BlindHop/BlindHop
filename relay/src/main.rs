//! BlindHop Relay — WebSocket Sphinx relay node.
//!
//! A relay node listens for incoming Sphinx packets, decrypts one layer,
//! and forwards to the next hop (or delivers at exit).

mod node;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use blindhop_lib::config::PACKET_SIZE;
use blindhop_lib::sphinx::keys::RelayKeyPair;
use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

#[derive(Parser)]
#[command(name = "blindhop-relay", about = "BlindHop Sphinx relay node")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new relay key pair and save to file.
    GenerateKey {
        /// Output file path.
        #[arg(short, long, default_value = "relay.key")]
        output: PathBuf,
    },
    /// Run the relay node.
    Run {
        /// Listen address (e.g., 0.0.0.0:9401).
        #[arg(short, long)]
        listen: SocketAddr,

        /// Path to the secret key file.
        #[arg(short, long)]
        secret_key_file: PathBuf,

        /// Hop index for this relay (0-indexed position in the route).
        #[arg(long, default_value = "0")]
        hop_index: usize,

        /// Target RPC URL for exit relay mode (if set, this relay is the exit).
        #[arg(long)]
        target: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateKey { output } => {
            let kp = RelayKeyPair::generate();
            let secret_hex = blindhop_lib::sphinx::keys::hex::encode(&kp.secret_bytes());
            let public_hex = blindhop_lib::sphinx::keys::hex::encode(kp.public.as_bytes());
            std::fs::write(&output, &secret_hex)
                .with_context(|| format!("Failed to write key to {:?}", output))?;
            tracing::info!("Generated relay key pair");
            tracing::info!("  Public key: {}", public_hex);
            tracing::info!("  Secret key saved to: {:?}", output);
            println!("{}", public_hex);
        }
        Commands::Run {
            listen,
            secret_key_file,
            hop_index,
            target,
        } => {
            let secret_hex = std::fs::read_to_string(&secret_key_file)
                .with_context(|| format!("Failed to read key from {:?}", secret_key_file))?;
            let secret_bytes = blindhop_lib::sphinx::keys::hex::decode(secret_hex.trim())
                .map_err(|e| anyhow::anyhow!("Invalid hex key: {}", e))?;
            let mut key_arr = [0u8; 32];
            if secret_bytes.len() != 32 {
                anyhow::bail!("Secret key must be 32 bytes, got {}", secret_bytes.len());
            }
            key_arr.copy_from_slice(&secret_bytes);
            let relay_keys = RelayKeyPair::from_secret_bytes(key_arr);

            let public_hex = blindhop_lib::sphinx::keys::hex::encode(relay_keys.public.as_bytes());
            tracing::info!("Starting relay node");
            tracing::info!("  Listen: {}", listen);
            tracing::info!("  Public key: {}", public_hex);
            tracing::info!("  Hop index: {}", hop_index);
            if let Some(ref t) = target {
                tracing::info!("  Exit relay mode → target: {}", t);
            }

            run_relay(listen, relay_keys, hop_index, target).await?;
        }
    }

    Ok(())
}

async fn run_relay(
    listen: SocketAddr,
    relay_keys: RelayKeyPair,
    hop_index: usize,
    target: Option<String>,
) -> Result<()> {
    let listener = TcpListener::bind(listen)
        .await
        .with_context(|| format!("Failed to bind to {}", listen))?;

    tracing::info!("Relay listening on ws://{}", listen);

    let relay_keys = Arc::new(relay_keys);
    let conn_pool = node::new_conn_pool();
    let target = target.map(Arc::new);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::debug!("New connection from {}", peer_addr);

        let relay_keys = relay_keys.clone();
        let conn_pool = conn_pool.clone();
        let target = target.clone();

        tokio::spawn(async move {
            let ws = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    tracing::warn!("WebSocket handshake failed from {}: {}", peer_addr, e);
                    return;
                }
            };

            let (mut ws_tx, mut ws_rx) = ws.split();

            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        if data.len() != PACKET_SIZE {
                            tracing::warn!(
                                "Invalid packet size from {}: {} (expected {})",
                                peer_addr,
                                data.len(),
                                PACKET_SIZE,
                            );
                            continue;
                        }

                        let target_ref = target.as_deref().map(|s| s.as_str());
                        match node::process_sphinx_packet(
                            &data,
                            &relay_keys,
                            hop_index,
                            &conn_pool,
                            target_ref,
                        )
                        .await
                        {
                            Ok(Some(reply)) => {
                                // Send reply back (for exit relay)
                                if let Err(e) =
                                    ws_tx.send(Message::Binary(reply.into())).await
                                {
                                    tracing::warn!("Failed to send reply to {}: {}", peer_addr, e);
                                }
                            }
                            Ok(None) => {
                                // Forwarded, no reply needed
                            }
                            Err(e) => {
                                tracing::warn!("Error processing packet from {}: {}", peer_addr, e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::debug!("Connection closed from {}", peer_addr);
                        break;
                    }
                    Ok(_) => {} // Ignore text, ping, pong
                    Err(e) => {
                        tracing::debug!("Connection error from {}: {}", peer_addr, e);
                        break;
                    }
                }
            }
        });
    }
}
