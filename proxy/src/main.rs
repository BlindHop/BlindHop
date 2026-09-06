//! BlindHop Proxy — Local WebSocket proxy that bridges smoldot traffic
//! through the Sphinx relay path.
//!
//! Smoldot connects to this proxy (thinks it's a full node).
//! The proxy Sphinx-wraps every message and routes through relays.

mod bridge;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use blindhop_lib::sphinx::keys::{NodeInfo, PublicKey};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

use bridge::{connect_to_entry_relay, send_through_relay_path, RelayPath};

#[derive(Parser)]
#[command(name = "blindhop-proxy", about = "BlindHop local WebSocket proxy")]
struct Cli {
    /// Listen address for incoming smoldot connections.
    #[arg(short, long, default_value = "127.0.0.1:9500")]
    listen: SocketAddr,

    /// Comma-separated relay path addresses (e.g., "ws://127.0.0.1:9401,ws://127.0.0.1:9402").
    #[arg(long)]
    relay_path: String,

    /// Comma-separated relay public keys in hex (same order as relay_path).
    #[arg(long)]
    relay_keys: String,

    /// Target full node WebSocket URL.
    #[arg(long)]
    target: String,

    /// Target full node public key in hex (for Sphinx destination).
    #[arg(long)]
    target_key: String,
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

    // Parse relay path
    let relay_addrs: Vec<&str> = cli.relay_path.split(',').collect();
    let relay_key_hexes: Vec<&str> = cli.relay_keys.split(',').collect();

    if relay_addrs.len() != relay_key_hexes.len() {
        anyhow::bail!(
            "Number of relay addresses ({}) must match number of relay keys ({})",
            relay_addrs.len(),
            relay_key_hexes.len()
        );
    }

    let mut nodes = Vec::new();
    for (addr_str, key_hex) in relay_addrs.iter().zip(relay_key_hexes.iter()) {
        let addr_str = addr_str.trim();
        let key_hex = key_hex.trim();

        // Parse address from ws://host:port format
        let addr: SocketAddr = addr_str
            .trim_start_matches("ws://")
            .trim_start_matches("wss://")
            .parse()
            .with_context(|| format!("Invalid relay address: {}", addr_str))?;

        let key_bytes = blindhop_lib::sphinx::keys::hex::decode(key_hex)
            .map_err(|e| anyhow::anyhow!("Invalid relay key hex: {}", e))?;
        let mut key_arr = [0u8; 32];
        if key_bytes.len() != 32 {
            anyhow::bail!("Relay key must be 32 bytes");
        }
        key_arr.copy_from_slice(&key_bytes);

        nodes.push(NodeInfo {
            public_key: PublicKey::from(key_arr),
            address: addr,
        });
    }

    // Parse target
    let target_key_bytes = blindhop_lib::sphinx::keys::hex::decode(cli.target_key.trim())
        .map_err(|e| anyhow::anyhow!("Invalid target key hex: {}", e))?;
    let mut target_key_arr = [0u8; 32];
    target_key_arr.copy_from_slice(&target_key_bytes);

    // For the target, use a dummy address (the exit relay handles actual forwarding)
    let target_node = NodeInfo {
        public_key: PublicKey::from(target_key_arr),
        address: "0.0.0.0:0".parse().unwrap(),
    };

    let relay_path = Arc::new(RelayPath {
        nodes,
        target: target_node,
    });

    let entry_relay_url = relay_addrs[0].trim().to_string();

    tracing::info!("Starting BlindHop proxy");
    tracing::info!("  Listen: {}", cli.listen);
    tracing::info!("  Relay path: {} hops", relay_path.nodes.len());
    tracing::info!("  Target: {}", cli.target);

    let listener = TcpListener::bind(cli.listen)
        .await
        .with_context(|| format!("Failed to bind to {}", cli.listen))?;

    tracing::info!("Proxy listening on ws://{}", cli.listen);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::info!("Smoldot connected from {}", peer_addr);

        let relay_path = relay_path.clone();
        let entry_url = entry_relay_url.clone();

        tokio::spawn(async move {
            let ws = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    tracing::warn!("WebSocket handshake failed: {}", e);
                    return;
                }
            };

            let (mut ws_tx, mut ws_rx) = ws.split();

            // Connect to entry relay
            let mut entry_ws = match connect_to_entry_relay(&entry_url).await {
                Ok(ws) => ws,
                Err(e) => {
                    tracing::error!("Failed to connect to entry relay: {}", e);
                    return;
                }
            };

            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // JSON-RPC from smoldot → Sphinx wrap → relay path
                        match send_through_relay_path(
                            text.as_bytes(),
                            &relay_path,
                            &mut entry_ws,
                        )
                        .await
                        {
                            Ok(reply) => {
                                let reply_text = String::from_utf8_lossy(&reply).to_string();
                                tracing::debug!("Got reply from relay path ({} bytes): {}", reply.len(), &reply_text[..reply_text.len().min(200)]);
                                if let Err(e) = ws_tx.send(Message::Text(reply_text.into())).await {
                                    tracing::warn!("Failed to send reply to smoldot: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Relay path error: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(e) => {
                        tracing::debug!("Client disconnected: {}", e);
                        break;
                    }
                }
            }
        });
    }
}
