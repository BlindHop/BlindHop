//! WebSocket client for communicating with Substrate full nodes.
//!
//! Keeps a pool of open connections so requests don't each pay for a new
//! TCP + TLS handshake. Each pooled connection carries one request at a
//! time, and the response is matched by JSON-RPC `id`.

use std::sync::Mutex;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::error::{CapacityError, Error as WsError};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config, tungstenite::Message,
};

use crate::policy::MAX_RESPONSE_BYTES;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How long to wait for the full node to answer one request.
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(30);

/// Idle connections kept open for reuse.
const MAX_IDLE_CONNECTIONS: usize = 8;

/// Why a request to the full node failed.
#[derive(Debug, PartialEq)]
pub enum UpstreamError {
    Connect(String),
    /// The connection closed or broke before the response arrived.
    Disconnected(String),
    TooLarge,
    Timeout,
}

impl std::fmt::Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(e) => write!(f, "Could not reach the full node: {}", e),
            Self::Disconnected(e) => write!(f, "Full node connection lost: {}", e),
            Self::TooLarge => write!(f, "Response too large (limit {} bytes)", MAX_RESPONSE_BYTES),
            Self::Timeout => write!(
                f,
                "Full node did not respond within {}s",
                UPSTREAM_TIMEOUT.as_secs()
            ),
        }
    }
}

/// Pool of WebSocket connections to one full node.
pub struct UpstreamPool {
    url: String,
    idle: Mutex<Vec<Ws>>,
}

impl UpstreamPool {
    pub fn new(url: String) -> Self {
        Self {
            url,
            idle: Mutex::new(Vec::new()),
        }
    }

    /// Send one JSON-RPC request (with id `id`) and return the response.
    pub async fn request(&self, request: &[u8], id: &Value) -> Result<Vec<u8>, UpstreamError> {
        let text = String::from_utf8_lossy(request).into_owned();

        if let Some(mut ws) = self.take_idle() {
            match exchange(&mut ws, &text, id).await {
                Ok(response) => {
                    self.put_idle(ws);
                    return Ok(response);
                }
                // The node may have closed the connection while it sat idle.
                // Retry once on a fresh one (resubmitting an extrinsic is
                // harmless: the node rejects the duplicate).
                Err(UpstreamError::Disconnected(e)) => {
                    tracing::debug!("Pooled connection was stale ({}); reconnecting", e);
                }
                Err(e) => return Err(e),
            }
        }

        let mut ws = self.connect().await?;
        let response = exchange(&mut ws, &text, id).await?;
        self.put_idle(ws);
        Ok(response)
    }

    async fn connect(&self) -> Result<Ws, UpstreamError> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_RESPONSE_BYTES))
            .max_frame_size(Some(MAX_RESPONSE_BYTES));
        let connect = connect_async_with_config(self.url.as_str(), Some(config), true);
        match tokio::time::timeout(UPSTREAM_TIMEOUT, connect).await {
            Ok(Ok((ws, _))) => Ok(ws),
            Ok(Err(e)) => Err(UpstreamError::Connect(e.to_string())),
            Err(_) => Err(UpstreamError::Timeout),
        }
    }

    fn take_idle(&self) -> Option<Ws> {
        self.idle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop()
    }

    fn put_idle(&self, ws: Ws) {
        let mut idle = self
            .idle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if idle.len() < MAX_IDLE_CONNECTIONS {
            idle.push(ws);
        }
    }
}

/// Send `text` and wait for the response whose `id` matches.
///
/// On any error or timeout the connection must be dropped by the caller
/// (never pooled), so a late response can't be read as another request's.
async fn exchange(ws: &mut Ws, text: &str, id: &Value) -> Result<Vec<u8>, UpstreamError> {
    let work = async {
        ws.send(Message::Text(text.into()))
            .await
            .map_err(|e| UpstreamError::Disconnected(e.to_string()))?;

        loop {
            let bytes = match ws.next().await {
                Some(Ok(Message::Text(t))) => t.as_bytes().to_vec(),
                Some(Ok(Message::Binary(b))) => b.to_vec(),
                Some(Ok(Message::Close(_))) | None => {
                    return Err(UpstreamError::Disconnected("closed".to_string()));
                }
                Some(Ok(_)) => continue, // ping/pong/frames
                Some(Err(WsError::Capacity(CapacityError::MessageTooLong { .. }))) => {
                    return Err(UpstreamError::TooLarge);
                }
                Some(Err(e)) => return Err(UpstreamError::Disconnected(e.to_string())),
            };

            let matches = serde_json::from_slice::<Value>(&bytes)
                .ok()
                .is_some_and(|v| v.get("id") == Some(id));
            if matches {
                return Ok(bytes);
            }
            tracing::debug!("Ignoring full-node message not matching id {}", id);
        }
    };

    tokio::time::timeout(UPSTREAM_TIMEOUT, work)
        .await
        .unwrap_or(Err(UpstreamError::Timeout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;

    /// Local WebSocket "full node". `respond` maps a request to the list of
    /// messages to send back. Returns the URL and a connection counter.
    async fn mock_node<F>(respond: F) -> (String, Arc<AtomicUsize>)
    where
        F: Fn(Value) -> Vec<String> + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let connections = Arc::new(AtomicUsize::new(0));
        let respond = Arc::new(respond);
        let counter = connections.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                let respond = respond.clone();
                tokio::spawn(async move {
                    let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                    while let Some(Ok(Message::Text(t))) = ws.next().await {
                        let req: Value = serde_json::from_str(&t).unwrap();
                        for out in respond(req) {
                            if ws.send(Message::Text(out.into())).await.is_err() {
                                return;
                            }
                        }
                    }
                });
            }
        });
        (url, connections)
    }

    fn echo_result(req: &Value, result: &str) -> String {
        serde_json::json!({ "jsonrpc": "2.0", "id": req["id"], "result": result }).to_string()
    }

    fn req(id: u64) -> (Vec<u8>, Value) {
        let body = serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": "system_chain" });
        (serde_json::to_vec(&body).unwrap(), serde_json::json!(id))
    }

    #[tokio::test]
    async fn connections_are_reused() {
        let (url, connections) = mock_node(|r| vec![echo_result(&r, "ok")]).await;
        let pool = UpstreamPool::new(url);
        for i in 0..5 {
            let (body, id) = req(i);
            let resp: Value =
                serde_json::from_slice(&pool.request(&body, &id).await.unwrap()).unwrap();
            assert_eq!(resp["id"], i);
        }
        assert_eq!(connections.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn messages_with_other_ids_are_skipped() {
        // A stale reply and a notification arrive before the real response.
        let (url, _) = mock_node(|r| {
            vec![
                serde_json::json!({ "jsonrpc": "2.0", "id": 999, "result": "stale" }).to_string(),
                serde_json::json!({ "jsonrpc": "2.0", "method": "note", "params": {} }).to_string(),
                echo_result(&r, "mine"),
            ]
        })
        .await;
        let pool = UpstreamPool::new(url);
        let (body, id) = req(42);
        let resp: Value = serde_json::from_slice(&pool.request(&body, &id).await.unwrap()).unwrap();
        assert_eq!(resp["result"], "mine");
    }

    #[tokio::test]
    async fn oversized_response_is_an_error() {
        let big = "x".repeat(MAX_RESPONSE_BYTES + 1);
        let (url, _) = mock_node(move |r| vec![echo_result(&r, &big)]).await;
        let pool = UpstreamPool::new(url);
        let (body, id) = req(1);
        assert_eq!(pool.request(&body, &id).await, Err(UpstreamError::TooLarge));
    }

    #[tokio::test]
    async fn stale_pooled_connection_is_retried_once() {
        // The node answers, then closes the connection after each response.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                    if let Some(Ok(Message::Text(t))) = ws.next().await {
                        let r: Value = serde_json::from_str(&t).unwrap();
                        let _ = ws.send(Message::Text(echo_result(&r, "ok").into())).await;
                    }
                    let _ = ws.close(None).await;
                });
            }
        });
        let pool = UpstreamPool::new(url);
        for i in 0..3 {
            let (body, id) = req(i);
            assert!(pool.request(&body, &id).await.is_ok(), "request {i}");
        }
    }

    #[tokio::test]
    async fn unreachable_node_is_a_connect_error() {
        let pool = UpstreamPool::new("ws://127.0.0.1:9".to_string());
        let (body, id) = req(1);
        assert!(matches!(
            pool.request(&body, &id).await,
            Err(UpstreamError::Connect(_))
        ));
    }
}
