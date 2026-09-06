//! Bridge: intercepts WebSocket messages from smoldot, wraps in Sphinx packets,
//! sends through the relay path, and returns responses.

use std::sync::Arc;

use anyhow::{Context, Result};
use blindhop_lib::config::PACKET_SIZE;
use blindhop_lib::sphinx::keys::NodeInfo;
use blindhop_lib::sphinx::packet::{prefix_payload, SphinxPacket};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

/// Relay path configuration.
#[derive(Clone)]
pub struct RelayPath {
    /// Ordered list of relay nodes in the path.
    pub nodes: Vec<NodeInfo>,
    /// The target full node (exit relay forwards here).
    pub target: NodeInfo,
}

/// Process a single JSON-RPC message through the relay path.
///
/// 1. Wrap the message in a Sphinx packet
/// 2. Send to entry relay via WebSocket
/// 3. Wait for reply (exit relay forwards to target, gets response, sends back)
/// 4. Return the response
pub async fn send_through_relay_path(
    message: &[u8],
    relay_path: &RelayPath,
    entry_ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<Vec<u8>> {
    // Prefix payload with length
    let prefixed = prefix_payload(message);

    // Create Sphinx packet
    let (packet, _keys) = SphinxPacket::create(&prefixed, &relay_path.nodes, &relay_path.target)
        .map_err(|e| anyhow::anyhow!("Failed to create Sphinx packet: {}", e))?;

    let packet_bytes = packet.to_bytes();
    assert_eq!(packet_bytes.len(), PACKET_SIZE);

    // Send to entry relay
    entry_ws
        .send(Message::Binary(packet_bytes.to_vec().into()))
        .await
        .context("Failed to send Sphinx packet to entry relay")?;

    // Wait for reply (skip Ping/Pong)
    loop {
        match entry_ws.next().await {
            Some(Ok(Message::Binary(data))) => return Ok(data.to_vec()),
            Some(Ok(Message::Text(text))) => return Ok(text.as_bytes().to_vec()),
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) | Some(Ok(Message::Frame(_))) => continue,
            Some(Ok(Message::Close(_))) => anyhow::bail!("Relay connection closed before reply"),
            Some(Err(e)) => anyhow::bail!("Error reading reply from relay path: {}", e),
            None => anyhow::bail!("Relay connection closed before reply"),
        }
    }
}

/// Connect to the entry relay.
pub async fn connect_to_entry_relay(
    entry_addr: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let (ws, _) = connect_async(entry_addr)
        .await
        .with_context(|| format!("Failed to connect to entry relay: {}", entry_addr))?;
    Ok(ws)
}

/// Direct mode: forward messages to target without Sphinx wrapping.
/// Used for baseline comparison.
pub async fn send_direct(
    message: &[u8],
    target_ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<Vec<u8>> {
    let text = String::from_utf8_lossy(message).to_string();
    target_ws
        .send(Message::Text(text.into()))
        .await
        .context("Failed to send direct message")?;

    if let Some(msg) = target_ws.next().await {
        let msg = msg.context("Error reading direct response")?;
        match msg {
            Message::Text(t) => Ok(t.as_bytes().to_vec()),
            Message::Binary(b) => Ok(b.to_vec()),
            _ => Ok(Vec::new()),
        }
    } else {
        anyhow::bail!("Direct connection closed before reply")
    }
}
