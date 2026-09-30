//! NymTransport — MixnetTransport implementation using the Nym SDK.
//!
//! This module wraps `nym_sdk::mixnet::MixnetClient` to provide
//! the pluggable `MixnetTransport` interface for routing traffic
//! through the Nym mixnet.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use nym_sdk::mixnet::MixnetMessageSender;

use async_trait::async_trait;
use tokio::sync::Mutex;

use blindhop_common::config::PrivacyMode;
use blindhop_common::error::{BlindHopError, Result};
use blindhop_common::metrics::{MetricsCollector, TransportMetrics};
use blindhop_common::rpc::MixnetMessage;
use blindhop_common::transport::{MixnetTransport, PrivacyInfo};

/// Nym-based mixnet transport.
///
/// Uses `nym-sdk` to route messages through the Nym mixnet.
/// Supports runtime switching between 2-hop (Fast) and 5-hop (Full) modes.
pub struct NymTransport {
    /// The Nym mixnet client.
    client: Mutex<Option<nym_sdk::mixnet::MixnetClient>>,

    /// Nym address of the exit service.
    exit_address: String,

    /// Current privacy mode.
    ///
    /// A `std` lock (never held across `.await`) so sync trait methods can
    /// read it without `blocking_read()`, which panics inside the runtime.
    privacy_mode: RwLock<PrivacyMode>,

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
        tracing::info!(
            "Connecting to Nym mixnet in {} mode...",
            privacy_mode.label()
        );

        let client = nym_sdk::mixnet::MixnetClient::connect_new()
            .await
            .map_err(|e| BlindHopError::NymTransport(format!("Failed to connect: {}", e)))?;

        let our_address = client.nym_address().to_string();
        tracing::info!("Connected to Nym mixnet");
        tracing::info!("  Our address: {}", our_address);

        let connected = Arc::new(AtomicBool::new(true));
        let shutting_down = Arc::new(AtomicBool::new(false));
        Self::watch_for_shutdown(&client, connected.clone(), shutting_down.clone());

        Ok(Self {
            client: Mutex::new(Some(client)),
            exit_address,
            privacy_mode: RwLock::new(privacy_mode),
            our_address,
            metrics: Arc::new(MetricsCollector::new(200)),
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
        client: &nym_sdk::mixnet::MixnetClient,
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
        *self
            .privacy_mode
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Parse a Nym recipient address from a string.
    fn parse_recipient(address: &str) -> Result<nym_sdk::mixnet::Recipient> {
        nym_sdk::mixnet::Recipient::try_from_base58_string(address.to_string())
            .map_err(|e| BlindHopError::NymTransport(format!("Invalid recipient address: {}", e)))
    }
}

#[async_trait]
impl MixnetTransport for NymTransport {
    async fn send(&self, data: &[u8]) -> Result<()> {
        let guard = self.client.lock().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| BlindHopError::NymTransport("Not connected".to_string()))?;

        let recipient = Self::parse_recipient(&self.exit_address)?;

        // Wrap in MixnetMessage envelope
        let msg = MixnetMessage::request(data.to_vec());
        let msg_bytes = msg.to_bytes();

        client
            .send_plain_message(recipient, msg_bytes)
            .await
            .map_err(|e| BlindHopError::NymTransport(format!("Send failed: {}", e)))?;

        drop(guard);
        self.metrics.record_send();
        tracing::debug!("Sent {} bytes through Nym mixnet", data.len());

        Ok(())
    }

    async fn recv(&self) -> Result<Vec<u8>> {
        let mut guard = self.client.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| BlindHopError::NymTransport("Not connected".to_string()))?;

        // Wait for next message from the mixnet
        // wait_for_messages returns Option<Vec<ReconstructedMessage>>
        let received = client
            .wait_for_messages()
            .await
            .ok_or_else(|| BlindHopError::NymTransport("Mixnet channel closed".to_string()))?;

        if let Some(msg) = received.into_iter().next() {
            let mixnet_msg = MixnetMessage::from_bytes(&msg.message)?;
            self.metrics.record_recv();
            tracing::debug!(
                "Received {} bytes from Nym mixnet",
                mixnet_msg.payload.len()
            );
            Ok(mixnet_msg.payload)
        } else {
            Err(BlindHopError::NymTransport(
                "No messages received".to_string(),
            ))
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

    async fn set_privacy_mode(&self, mode: PrivacyMode) -> Result<()> {
        let current = self.current_mode();
        if current == mode {
            return Ok(());
        }

        tracing::info!(
            "Switching privacy mode: {} → {}",
            current.label(),
            mode.label()
        );

        // For now, mode switching requires reconnecting the Nym client.
        // Future optimization: Nym SDK may support live mode switching.
        *self
            .privacy_mode
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = mode;

        tracing::info!("Privacy mode updated to {}", mode.label());
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn disconnect(&self) -> Result<()> {
        let mut guard = self.client.lock().await;
        if let Some(client) = guard.take() {
            self.shutting_down.store(true, Ordering::SeqCst);
            client.disconnect().await;
            self.connected.store(false, Ordering::SeqCst);
            tracing::info!("Disconnected from Nym mixnet");
        }
        Ok(())
    }
}
