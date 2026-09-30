//! Bridge: accepts WebSocket connections from smoldot and routes traffic
//! through the appropriate transport based on the privacy mode.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{StatusCode, header::ORIGIN};

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
    let check_origin = |req: &Request, resp: Response| {
        // A present but non-UTF-8 Origin is treated as disallowed.
        let origin = req.headers().get(ORIGIN).map(|v| v.to_str().unwrap_or(""));
        if origin_allowed(origin, &config.allowed_origins) {
            Ok(resp)
        } else {
            tracing::warn!("Refused WebSocket connection from origin {:?}", origin);
            let mut refusal = ErrorResponse::new(Some("Origin not allowed".to_string()));
            *refusal.status_mut() = StatusCode::FORBIDDEN;
            Err(refusal)
        }
    };
    let ws = tokio_tungstenite::accept_hdr_async(stream, check_origin).await?;
    let (mut ws_tx, mut ws_rx) = ws.split();

    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let start = Instant::now();
                let data = text.as_bytes();

                // Check if this is a control message (privacy mode change)
                if let Ok(control) = serde_json::from_slice::<ControlMessage>(data) {
                    if control.method == "blindhop_setPrivacyMode" {
                        let reply = set_privacy_mode_reply(&control, &transport, &config).await;
                        let _ = ws_tx.send(Message::Text(reply.to_string().into())).await;
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
                                "mode_id": info.mode,
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

/// Whether a WebSocket handshake with this `Origin` header may connect.
///
/// Browsers attach `Origin` to every WebSocket handshake and apply no CORS
/// checks, so without this any web page the user visits could drive the
/// proxy (e.g. switch it to direct mode, exposing the user's IP).
/// Non-browser clients send no `Origin` and are allowed.
fn origin_allowed(origin: Option<&str>, allowed: &[String]) -> bool {
    match origin {
        None => true,
        Some(origin) => allowed
            .iter()
            .any(|a| a.trim_end_matches('/').eq_ignore_ascii_case(origin)),
    }
}

/// Handle `blindhop_setPrivacyMode` and build its JSON-RPC reply.
///
/// On success the reply reports the mode now in effect. On failure it is a
/// JSON-RPC error whose `data.mode_id` is the mode actually in effect, so a
/// UI never shows a privacy level the proxy isn't providing.
async fn set_privacy_mode_reply(
    control: &ControlMessage,
    transport: &Arc<RwLock<ActiveTransport>>,
    config: &BlindHopConfig,
) -> serde_json::Value {
    let error = |code: i64, message: String, current: Option<PrivacyMode>| {
        let mut error = serde_json::json!({ "code": code, "message": message });
        if let Some(mode) = current {
            error["data"] = serde_json::json!({ "mode": mode.label(), "mode_id": mode });
        }
        serde_json::json!({ "jsonrpc": "2.0", "id": control.id, "error": error })
    };

    let new_mode = match parse_mode_param(&control.params) {
        Ok(mode) => mode,
        Err(message) => return error(-32602, message, None),
    };
    tracing::info!("Privacy mode change requested: {}", new_mode.label());

    let mut guard = transport.write().await;
    match guard.switch_mode(new_mode, config).await {
        Ok(()) => serde_json::json!({
            "jsonrpc": "2.0",
            "id": control.id,
            "result": { "mode": new_mode.label(), "mode_id": new_mode, "status": "ok" }
        }),
        Err(e) => {
            tracing::error!("Failed to switch privacy mode: {}", e);
            let current = guard.privacy_info().mode;
            error(
                -32000,
                format!("Failed to switch privacy mode: {}", e),
                Some(current),
            )
        }
    }
}

/// Parse `["none" | "fast" | "full"]`.
fn parse_mode_param(params: &[serde_json::Value]) -> Result<PrivacyMode, String> {
    match params.first().and_then(|v| v.as_str()) {
        Some("none") => Ok(PrivacyMode::None),
        Some("fast") => Ok(PrivacyMode::Fast),
        Some("full") => Ok(PrivacyMode::Full),
        Some(other) => Err(format!("Unknown privacy mode: {}", other)),
        None => Err(r#"Expected params: ["none" | "fast" | "full"]"#.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed() -> Vec<String> {
        vec![
            "https://demo.blindhop.wtf".to_string(),
            "http://127.0.0.1:8080/".to_string(),
        ]
    }

    #[test]
    fn clients_without_origin_are_allowed() {
        assert!(origin_allowed(None, &[]));
    }

    #[test]
    fn unlisted_origins_are_refused() {
        assert!(!origin_allowed(Some("https://evil.example"), &allowed()));
        assert!(!origin_allowed(Some("https://evil.example"), &[]));
        assert!(!origin_allowed(Some("null"), &allowed()));
        assert!(!origin_allowed(Some(""), &allowed()));
        assert!(!origin_allowed(
            Some("https://demo.blindhop.wtf.evil.example"),
            &allowed()
        ));
    }

    #[test]
    fn mode_param_parsing() {
        use serde_json::json;
        assert_eq!(parse_mode_param(&[json!("full")]), Ok(PrivacyMode::Full));
        assert_eq!(parse_mode_param(&[json!("none")]), Ok(PrivacyMode::None));
        assert!(parse_mode_param(&[json!("max")]).is_err());
        assert!(parse_mode_param(&[json!(5)]).is_err());
        assert!(parse_mode_param(&[]).is_err());
    }

    fn control(params: serde_json::Value) -> ControlMessage {
        serde_json::from_value(serde_json::json!({
            "id": 7, "method": "blindhop_setPrivacyMode", "params": params
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn failed_switch_reports_error_and_actual_mode() {
        // Direct mode needs no network. Switching to Full without an exit
        // address fails, and the proxy stays in Direct mode.
        let config = BlindHopConfig {
            privacy_mode: PrivacyMode::None,
            exit_address: None,
            ..Default::default()
        };
        let transport = Arc::new(RwLock::new(ActiveTransport::new(&config).await.unwrap()));

        let reply =
            set_privacy_mode_reply(&control(serde_json::json!(["full"])), &transport, &config)
                .await;
        assert_eq!(reply["id"], 7);
        assert!(
            reply.get("result").is_none(),
            "must not report success: {reply}"
        );
        assert_eq!(reply["error"]["code"], -32000);
        assert_eq!(reply["error"]["data"]["mode_id"], "none");
    }

    #[tokio::test]
    async fn successful_switch_reports_new_mode() {
        let config = BlindHopConfig {
            privacy_mode: PrivacyMode::None,
            ..Default::default()
        };
        let transport = Arc::new(RwLock::new(ActiveTransport::new(&config).await.unwrap()));

        // None -> None is a no-op switch that succeeds without network.
        let reply =
            set_privacy_mode_reply(&control(serde_json::json!(["none"])), &transport, &config)
                .await;
        assert_eq!(reply["result"]["mode_id"], "none");
        assert_eq!(reply["result"]["status"], "ok");

        let reply =
            set_privacy_mode_reply(&control(serde_json::json!(["bogus"])), &transport, &config)
                .await;
        assert_eq!(reply["error"]["code"], -32602);
    }

    #[test]
    fn listed_origins_are_allowed() {
        assert!(origin_allowed(
            Some("https://demo.blindhop.wtf"),
            &allowed()
        ));
        assert!(origin_allowed(
            Some("https://DEMO.blindhop.wtf"),
            &allowed()
        ));
        // A trailing slash in the configured origin is ignored.
        assert!(origin_allowed(Some("http://127.0.0.1:8080"), &allowed()));
    }
}
