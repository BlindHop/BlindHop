//! Bridge: accepts WebSocket connections from smoldot and routes traffic
//! through the appropriate transport based on the privacy mode.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;

use blindhop_common::config::{BlindHopConfig, PrivacyMode};
use blindhop_common::metrics::MetricsCollector;

use crate::mode::ActiveTransport;

/// Run the proxy server, accepting smoldot connections and routing
/// through the configured transport.
pub async fn run_proxy(listener: TcpListener, config: BlindHopConfig) -> Result<()> {
    let config = Arc::new(config);
    let metrics = Arc::new(MetricsCollector::new(200));

    // Initialize the transport based on privacy mode
    let transport = ActiveTransport::new(&config).await?;
    let transport = Arc::new(RwLock::new(transport));

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::info!("Smoldot connected from {}", peer_addr);

        let config = config.clone();
        let transport = transport.clone();
        let metrics = metrics.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, config, transport, metrics).await {
                tracing::warn!("Connection from {} ended with error: {}", peer_addr, e);
            }
        });
    }
}

/// Handle a single smoldot WebSocket connection.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    config: Arc<BlindHopConfig>,
    transport: Arc<RwLock<ActiveTransport>>,
    metrics: Arc<MetricsCollector>,
) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_tx, mut ws_rx) = ws.split();

    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let start = Instant::now();
                let data = text.as_bytes();

                // Check if this is a control message (privacy mode change)
                if let Ok(control) = serde_json::from_slice::<ControlMessage>(data) {
                    if control.method == "blindhop_setPrivacyMode" {
                        handle_control_message(&control, &transport, &config).await;
                        // Send acknowledgment
                        let ack = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": control.id,
                            "result": {
                                "mode": format!("{}", config.privacy_mode),
                                "status": "ok"
                            }
                        });
                        let _ = ws_tx.send(Message::Text(ack.to_string().into())).await;
                        continue;
                    }

                    if control.method == "blindhop_getMetrics" {
                        let guard = transport.read().await;
                        let info = guard.privacy_info();
                        let transport_metrics = guard.transport_metrics();
                        let ack = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": control.id,
                            "result": {
                                "mode": format!("{}", info.mode),
                                "hop_count": info.hop_count,
                                "cover_traffic": info.cover_traffic_active,
                                "gateway": info.gateway_address,
                                "our_address": info.our_address,
                                "latency_p50": transport_metrics.latency_p50_ms,
                                "latency_p95": transport_metrics.latency_p95_ms,
                                "messages_sent": transport_metrics.messages_sent,
                                "messages_received": transport_metrics.messages_received,
                            }
                        });
                        let _ = ws_tx.send(Message::Text(ack.to_string().into())).await;
                        continue;
                    }
                }

                // Route through transport based on privacy mode
                let reply = {
                    let guard = transport.read().await;
                    guard.send_and_recv(data).await
                };

                let elapsed = start.elapsed().as_millis() as f64;
                metrics.record_latency(elapsed);

                match reply {
                    Ok(response) => {
                        let reply_text = String::from_utf8_lossy(&response).to_string();
                        tracing::debug!("Response ({} bytes, {:.0}ms)", response.len(), elapsed);
                        if let Err(e) = ws_tx.send(Message::Text(reply_text.into())).await {
                            tracing::warn!("Failed to send reply to smoldot: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Transport error: {}", e);
                        // Send JSON-RPC error back to smoldot
                        let error_resp = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {
                                "code": -32000,
                                "message": format!("BlindHop transport error: {}", e)
                            }
                        });
                        let _ = ws_tx
                            .send(Message::Text(error_resp.to_string().into()))
                            .await;
                    }
                }
            }
            Ok(Message::Binary(data)) => {
                // Forward binary messages the same way
                let guard = transport.read().await;
                if let Ok(response) = guard.send_and_recv(&data).await {
                    let _ = ws_tx.send(Message::Binary(response.into())).await;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                let _ = ws_tx.send(Message::Pong(data)).await;
            }
            Ok(_) => {} // Skip Pong and Frame messages
            Err(e) => {
                tracing::debug!("Client disconnected: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle a BlindHop control message (e.g., privacy mode change).
async fn handle_control_message(
    control: &ControlMessage,
    transport: &Arc<RwLock<ActiveTransport>>,
    config: &BlindHopConfig,
) {
    if let Some(mode_str) = control.params.first().and_then(|v| v.as_str()) {
        let new_mode = match mode_str {
            "none" => PrivacyMode::None,
            "fast" => PrivacyMode::Fast,
            "full" => PrivacyMode::Full,
            _ => {
                tracing::warn!("Unknown privacy mode: {}", mode_str);
                return;
            }
        };

        tracing::info!("Privacy mode change requested: {}", new_mode.label());

        let mut guard = transport.write().await;
        if let Err(e) = guard.switch_mode(new_mode, config).await {
            tracing::error!("Failed to switch privacy mode: {}", e);
        }
    }
}

/// Control message from the demo UI.
#[derive(Debug, serde::Deserialize)]
struct ControlMessage {
    #[serde(default)]
    id: Option<u64>,
    method: String,
    #[serde(default)]
    params: Vec<serde_json::Value>,
}
