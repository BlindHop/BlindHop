//! WebSocket client for communicating with Substrate full nodes.
//!
//! Handles sending JSON-RPC requests and receiving responses
//! over WebSocket connections to the target chain.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use blindhop_common::error::{BlindHopError, Result};

/// Forward a raw JSON-RPC message to a Substrate full node and return the response.
///
/// Opens a new WebSocket connection for each request. In production,
/// this should be replaced with a connection pool for better performance.
pub async fn forward_rpc_message(request: &[u8], target_url: &str) -> Result<Vec<u8>> {
    let (mut ws, _) = connect_async(target_url).await.map_err(|e| {
        BlindHopError::SubstrateRpc(format!("Failed to connect to {}: {}", target_url, e))
    })?;

    let text = String::from_utf8_lossy(request).to_string();
    ws.send(Message::Text(text.into()))
        .await
        .map_err(|e| BlindHopError::SubstrateRpc(format!("Send failed: {}", e)))?;

    // Wait for response, skipping WebSocket control frames
    loop {
        match ws.next().await {
            Some(Ok(Message::Text(t))) => {
                let _ = ws.close(None).await;
                return Ok(t.as_bytes().to_vec());
            }
            Some(Ok(Message::Binary(b))) => {
                let _ = ws.close(None).await;
                return Ok(b.to_vec());
            }
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) => {
                return Err(BlindHopError::SubstrateRpc(
                    "Connection closed before response".to_string(),
                ));
            }
            Some(Err(e)) => {
                return Err(BlindHopError::SubstrateRpc(format!("Read error: {}", e)));
            }
            None => {
                return Err(BlindHopError::SubstrateRpc("Connection closed".to_string()));
            }
            _ => continue,
        }
    }
}
