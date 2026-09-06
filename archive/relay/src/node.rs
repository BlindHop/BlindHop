//! WebSocket relay node — accepts Sphinx packets, decrypts one layer, forwards.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use blindhop_lib::sphinx::packet::{RelayAction, SphinxPacket};
use blindhop_lib::sphinx::keys::RelayKeyPair;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};

/// Shared state for outbound connections (connection pooling).
type ConnPool = Arc<Mutex<HashMap<SocketAddr, WebSocketStream<MaybeTlsStream<TcpStream>>>>>;

/// Process a single incoming Sphinx packet.
///
/// Returns the action to take: forward to next hop or deliver payload.
pub async fn process_sphinx_packet(
    data: &[u8],
    relay_keys: &RelayKeyPair,
    hop_index: usize,
    conn_pool: &ConnPool,
    target_rpc: Option<&str>,
) -> Result<Option<Vec<u8>>> {
    let packet = SphinxPacket::from_bytes(data)
        .map_err(|e| anyhow::anyhow!("Invalid Sphinx packet: {}", e))?;

    let action = packet
        .process_at_relay(relay_keys, hop_index)
        .map_err(|e| anyhow::anyhow!("Relay processing failed: {}", e))?;

    match action {
        RelayAction::Forward { next_addr, packet } => {
            tracing::debug!("Forwarding to next hop: {}", next_addr);
            let packet_bytes = packet.to_bytes();

            // Get or create connection to next hop
            let ws_url = format!("ws://{}", next_addr);
            let mut pool = conn_pool.lock().await;

            // Try to send on existing connection, or create a new one
            let needs_new = !pool.contains_key(&next_addr);
            if needs_new {
                let (ws, _) = connect_async(&ws_url)
                    .await
                    .with_context(|| format!("Failed to connect to next hop: {}", ws_url))?;
                pool.insert(next_addr, ws);
            }

            if let Some(ws) = pool.get_mut(&next_addr) {
                ws.send(Message::Binary(packet_bytes.to_vec().into()))
                    .await
                    .with_context(|| format!("Failed to send to next hop: {}", next_addr))?;

                // Wait for reply from next hop and relay it back (skip Ping/Pong)
                loop {
                    match ws.next().await {
                        Some(Ok(Message::Binary(data))) => return Ok(Some(data.to_vec())),
                        Some(Ok(Message::Text(text))) => return Ok(Some(text.as_bytes().to_vec())),
                        Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                        _ => return Ok(None),
                    }
                }
            }

            Ok(None)
        }
        RelayAction::Deliver { payload } => {
            tracing::info!("Exit relay: delivering payload ({} bytes)", payload.len());

            // If we have a target RPC, forward the payload as a JSON-RPC request
            if let Some(target) = target_rpc {
                let reply = forward_to_target(target, &payload).await?;
                Ok(Some(reply))
            } else {
                // Echo mode: just return the payload
                Ok(Some(payload))
            }
        }
    }
}

/// Forward a payload to the target RPC endpoint and return the response.
async fn forward_to_target(target_url: &str, payload: &[u8]) -> Result<Vec<u8>> {
    let (mut ws, _) = connect_async(target_url)
        .await
        .with_context(|| format!("Failed to connect to target: {}", target_url))?;

    // Send the payload as a text message (JSON-RPC)
    let text = std::str::from_utf8(payload).unwrap_or_default().to_string();
    ws.send(Message::Text(text.into()))
        .await
        .context("Failed to send to target")?;

    // Wait for response (skip Ping/Pong)
    loop {
        match ws.next().await {
            Some(Ok(Message::Text(t))) => return Ok(t.as_bytes().to_vec()),
            Some(Ok(Message::Binary(b))) => return Ok(b.to_vec()),
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) => return Ok(Vec::new()),
            Some(Err(e)) => anyhow::bail!("Error reading target response: {}", e),
            _ => return Ok(Vec::new()),
        }
    }
}

/// Create a new connection pool.
pub fn new_conn_pool() -> ConnPool {
    Arc::new(Mutex::new(HashMap::new()))
}
