//! NymTransport — MixnetTransport implementation using the Nym SDK.
//!
//! This module wraps `nym_sdk::mixnet::MixnetClient` to provide
//! the pluggable `MixnetTransport` interface for routing traffic
//! through the Nym mixnet.
//!
//! One client is shared by every connection. Requests are sent through a
//! cloned sender, and a single background task owns the receive side,
//! routing each reply to its waiting request by correlation ID.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use nym_sdk::mixnet::{
    MixnetClient, MixnetClientBuilder, MixnetClientSender, MixnetMessageSender, Recipient,
};

use async_trait::async_trait;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use blindhop_common::config::PrivacyMode;
use blindhop_common::error::{BlindHopError, Result};
use blindhop_common::metrics::{MetricsCollector, TransportMetrics};
use blindhop_common::rpc::{MessageType, MixnetMessage};
use blindhop_common::transport::{MixnetTransport, PrivacyInfo};

/// How long a request waits for its reply before failing.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Requests awaiting a reply, keyed by correlation ID.
type PendingReplies = Arc<Mutex<HashMap<u64, oneshot::Sender<Vec<u8>>>>>;

/// Nym-based mixnet transport.
///
/// Uses `nym-sdk` to route messages through the Nym mixnet.
/// Supports runtime switching between 2-hop (Fast) and 5-hop (Full) modes.
pub struct NymTransport {
    /// Send half of the Nym client; the receive half lives in `receiver_task`.
    sender: MixnetClientSender,

    /// Nym address of the exit service.
    exit_recipient: Recipient,

    /// Requests awaiting a reply.
    pending: PendingReplies,

    /// Next correlation ID. Starts at 1; 0 means "no ID" on the wire.
    next_id: AtomicU64,

    /// Signals the receiver task to disconnect the client and stop.
    shutdown: Mutex<Option<oneshot::Sender<()>>>,

    /// Background task that owns the client and routes replies.
    receiver_task: Mutex<Option<JoinHandle<()>>>,

    /// Privacy mode the client was built for. Fixed for the client's
    /// lifetime; switching modes reconnects (see `ActiveTransport`).
    privacy_mode: PrivacyMode,

    /// Our Nym address.
    our_address: String,

    /// Metrics collector.
    metrics: Arc<MetricsCollector>,

    /// Whether the client is connected.
    connected: Arc<AtomicBool>,

    /// Set before an intentional disconnect, so the shutdown watcher
    /// doesn't treat it as a crash.
    shutting_down: Arc<AtomicBool>,
}

impl NymTransport {
    /// Create a new NymTransport and connect to the Nym mixnet.
    ///
    /// # Arguments
    ///
    /// * `exit_address` - The Nym address of the BlindHop exit service
    /// * `privacy_mode` - Initial privacy mode (Fast or Full)
    pub async fn connect(exit_address: String, privacy_mode: PrivacyMode) -> Result<Self> {
        let exit_recipient = Recipient::try_from_base58_string(&exit_address)
            .map_err(|e| BlindHopError::NymTransport(format!("Invalid exit address: {}", e)))?;

        tracing::info!(
            "Connecting to Nym mixnet in {} mode...",
            privacy_mode.label()
        );

        let client = MixnetClientBuilder::new_ephemeral()
            .debug_config(client_config(privacy_mode))
            .build()
            .map_err(|e| BlindHopError::NymTransport(format!("Failed to build client: {}", e)))?
            .connect_to_mixnet()
            .await
            .map_err(|e| BlindHopError::NymTransport(format!("Failed to connect: {}", e)))?;

        let our_address = client.nym_address().to_string();
        tracing::info!("Connected to Nym mixnet");
        tracing::info!("  Our address: {}", our_address);

        let connected = Arc::new(AtomicBool::new(true));
        let shutting_down = Arc::new(AtomicBool::new(false));
        Self::watch_for_shutdown(&client, connected.clone(), shutting_down.clone());

        let sender = client.split_sender();
        let pending = PendingReplies::default();
        let metrics = Arc::new(MetricsCollector::new(200));
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let receiver_task = tokio::spawn(receive_replies(
            client,
            pending.clone(),
            metrics.clone(),
            shutting_down.clone(),
            shutdown_rx,
        ));

        Ok(Self {
            sender,
            exit_recipient,
            pending,
            next_id: AtomicU64::new(1),
            shutdown: Mutex::new(Some(shutdown_tx)),
            receiver_task: Mutex::new(Some(receiver_task)),
            privacy_mode,
            our_address,
            metrics,
            connected,
            shutting_down,
        })
    }

    /// Exit the process if the Nym client shuts down unexpectedly.
    ///
    /// After an internal failure the SDK cancels its tasks but the process
    /// keeps running, failing every request. Exiting non-zero lets a
    /// supervisor (systemd, Docker) restart it instead.
    fn watch_for_shutdown(
        client: &MixnetClient,
        connected: Arc<AtomicBool>,
        shutting_down: Arc<AtomicBool>,
    ) {
        let token = client.cancellation_token();
        tokio::spawn(async move {
            token.cancelled().await;
            connected.store(false, Ordering::SeqCst);
            if !shutting_down.load(Ordering::SeqCst) {
                tracing::error!("Nym client shut down unexpectedly; exiting");
                std::process::exit(1);
            }
        });
    }

    fn current_mode(&self) -> PrivacyMode {
        self.privacy_mode
    }
}

/// Nym client settings that make each privacy mode what the UI says it is.
///
/// - Full: SDK defaults. Entry gateway, three mix nodes, exit gateway
///   (5 hops), with loop cover traffic and Poisson-delayed sending.
/// - Fast: entry gateway straight to exit gateway (2 hops), skipping the mix
///   nodes. This also applies to the reply SURBs handed to the exit, so
///   replies skip them too. No cover traffic or Poisson delays: lower
///   latency, weaker resistance to traffic analysis.
fn client_config(mode: PrivacyMode) -> nym_sdk::DebugConfig {
    let mut config = nym_sdk::DebugConfig::default();
    if mode == PrivacyMode::Fast {
        config.traffic.disable_mix_hops = true;
        config.cover_traffic.disable_loop_cover_traffic_stream = true;
        config.traffic.disable_main_poisson_packet_distribution = true;
    }
    config
}

/// Owns the client's receive side: routes each reply to the request with
/// the same correlation ID, until shutdown is signalled or the channel closes.
async fn receive_replies(
    mut client: MixnetClient,
    pending: PendingReplies,
    metrics: Arc<MetricsCollector>,
    shutting_down: Arc<AtomicBool>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            // Also fires if the transport is dropped without disconnect().
            _ = &mut shutdown_rx => break,
            batch = client.wait_for_messages() => {
                let Some(batch) = batch else {
                    tracing::warn!("Nym mixnet channel closed");
                    break;
                };
                for msg in batch {
                    route_reply(&msg.message, &pending, &metrics);
                }
            }
        }
    }

    // Dropping the waiters fails their requests immediately instead of
    // leaving them to time out.
    lock(&pending).clear();
    shutting_down.store(true, Ordering::SeqCst);
    client.disconnect().await;
}

/// Deliver one received message to the request waiting for it.
fn route_reply(bytes: &[u8], pending: &PendingReplies, metrics: &MetricsCollector) {
    let msg = match MixnetMessage::from_bytes(bytes) {
        Ok(msg) if msg.msg_type == MessageType::Response => msg,
        Ok(_) => {
            tracing::debug!("Ignoring non-response mixnet message");
            return;
        }
        Err(e) => {
            tracing::warn!("Dropping unparseable mixnet message: {}", e);
            return;
        }
    };

    let waiter = lock(pending).remove(&msg.correlation_id);
    match waiter {
        Some(tx) => {
            metrics.record_recv();
            tracing::debug!(
                "Received {} bytes from Nym mixnet (correlation ID {})",
                msg.payload.len(),
                msg.correlation_id
            );
            // The requester may have given up (timeout); nothing to do then.
            let _ = tx.send(msg.payload);
        }
        None => tracing::warn!(
            "Dropping reply with unknown correlation ID {} (timed out or duplicate)",
            msg.correlation_id
        ),
    }
}

/// Lock a mutex, ignoring poisoning (critical sections here can't panic
/// midway and leave the data inconsistent).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Removes a request's entry from the pending map when the request ends,
/// however it ends (reply, error, timeout, or the future being dropped).
struct PendingGuard<'a> {
    pending: &'a PendingReplies,
    id: u64,
}

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        lock(self.pending).remove(&self.id);
    }
}

#[async_trait]
impl MixnetTransport for NymTransport {
    async fn request(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(BlindHopError::NymTransport("Not connected".to_string()));
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        lock(&self.pending).insert(id, tx);
        let _guard = PendingGuard {
            pending: &self.pending,
            id,
        };

        let msg_bytes = MixnetMessage::request(id, data.to_vec())
            .accepting_compression()
            .to_bytes();
        self.sender
            .send_plain_message(self.exit_recipient, msg_bytes)
            .await
            .map_err(|e| BlindHopError::NymTransport(format!("Send failed: {}", e)))?;

        self.metrics.record_send();
        tracing::debug!(
            "Sent {} bytes through Nym mixnet (correlation ID {})",
            data.len(),
            id
        );

        match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(payload)) => Ok(payload),
            Ok(Err(_)) => Err(BlindHopError::NymTransport(
                "Nym client disconnected before the reply arrived".to_string(),
            )),
            Err(_) => Err(BlindHopError::NymTransport(format!(
                "No reply from exit within {}s",
                REQUEST_TIMEOUT.as_secs()
            ))),
        }
    }

    fn privacy_info(&self) -> PrivacyInfo {
        let mode = self.current_mode();

        PrivacyInfo {
            mode,
            hop_count: mode.hop_count(),
            cover_traffic_active: mode.has_cover_traffic(),
            gateway_address: None,    // TODO: Extract from Nym client
            anonymity_set_size: None, // TODO: Query from Nym network
            our_address: Some(self.our_address.clone()),
        }
    }

    fn metrics(&self) -> TransportMetrics {
        self.metrics.snapshot()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn disconnect(&self) -> Result<()> {
        let shutdown = lock(&self.shutdown).take();
        if let Some(shutdown) = shutdown {
            self.shutting_down.store(true, Ordering::SeqCst);
            let _ = shutdown.send(());
            let task = lock(&self.receiver_task).take();
            if let Some(task) = task {
                let _ = task.await;
            }
            self.connected.store(false, Ordering::SeqCst);
            tracing::info!("Disconnected from Nym mixnet");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_bytes(id: u64, payload: &[u8]) -> Vec<u8> {
        MixnetMessage::response(id, payload.to_vec()).to_bytes()
    }

    fn waiter(pending: &PendingReplies, id: u64) -> oneshot::Receiver<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        lock(pending).insert(id, tx);
        rx
    }

    #[test]
    fn replies_reach_their_own_request_in_any_order() {
        let pending = PendingReplies::default();
        let metrics = MetricsCollector::new(10);
        let mut rx1 = waiter(&pending, 1);
        let mut rx2 = waiter(&pending, 2);
        let mut rx3 = waiter(&pending, 3);

        route_reply(&response_bytes(3, b"three"), &pending, &metrics);
        route_reply(&response_bytes(1, b"one"), &pending, &metrics);
        route_reply(&response_bytes(2, b"two"), &pending, &metrics);

        assert_eq!(rx1.try_recv().unwrap(), b"one");
        assert_eq!(rx2.try_recv().unwrap(), b"two");
        assert_eq!(rx3.try_recv().unwrap(), b"three");
        assert!(lock(&pending).is_empty());
    }

    #[test]
    fn unknown_and_non_response_messages_are_dropped() {
        let pending = PendingReplies::default();
        let metrics = MetricsCollector::new(10);
        let mut rx = waiter(&pending, 1);

        route_reply(&response_bytes(99, b"stale"), &pending, &metrics);
        route_reply(
            &MixnetMessage::request(1, b"not a reply".to_vec()).to_bytes(),
            &pending,
            &metrics,
        );
        route_reply(b"not json", &pending, &metrics);

        assert!(rx.try_recv().is_err(), "waiter must still be pending");
        assert_eq!(lock(&pending).len(), 1);
    }

    #[test]
    fn fast_mode_skips_mix_nodes_and_cover_traffic() {
        let fast = client_config(PrivacyMode::Fast);
        assert!(fast.traffic.disable_mix_hops);
        assert!(fast.cover_traffic.disable_loop_cover_traffic_stream);
        assert!(fast.traffic.disable_main_poisson_packet_distribution);
    }

    #[test]
    fn full_mode_uses_mix_nodes_and_cover_traffic() {
        let full = client_config(PrivacyMode::Full);
        assert!(!full.traffic.disable_mix_hops);
        assert!(!full.cover_traffic.disable_loop_cover_traffic_stream);
        assert!(!full.traffic.disable_main_poisson_packet_distribution);
    }

    #[test]
    fn pending_guard_removes_entry_on_drop() {
        let pending = PendingReplies::default();
        let _rx = waiter(&pending, 7);
        {
            let _guard = PendingGuard {
                pending: &pending,
                id: 7,
            };
        }
        assert!(lock(&pending).is_empty());
    }
}
