//! Pluggable mixnet transport trait.
//!
//! This trait defines the interface for routing traffic through a mixnet.
//! The MVP implements [`NymTransport`](crate) (in `blindhop-proxy`).
//! Future implementations can add `SphinxRelayTransport` for custom mixnodes.

use async_trait::async_trait;

use crate::config::PrivacyMode;
use crate::error::Result;
use crate::metrics::TransportMetrics;

/// Information about the current privacy state of the transport.
#[derive(Debug, Clone)]
pub struct PrivacyInfo {
    /// Current privacy mode.
    pub mode: PrivacyMode,

    /// Number of hops in the current route.
    pub hop_count: u8,

    /// Whether cover traffic is currently being generated.
    pub cover_traffic_active: bool,

    /// Nym gateway address (if connected).
    pub gateway_address: Option<String>,

    /// Estimated anonymity set size.
    pub anonymity_set_size: Option<u32>,

    /// Our Nym address (if connected).
    pub our_address: Option<String>,
}

impl Default for PrivacyInfo {
    fn default() -> Self {
        Self {
            mode: PrivacyMode::None,
            hop_count: 0,
            cover_traffic_active: false,
            gateway_address: None,
            anonymity_set_size: None,
            our_address: None,
        }
    }
}

/// Pluggable mixnet transport trait.
///
/// Implementors provide the ability to send and receive messages through
/// a privacy-preserving mixnet. The transport handles all mixing, cover
/// traffic, and routing internally.
///
/// # MVP Implementation
///
/// The MVP provides `NymTransport` which uses the Nym SDK (`nym-sdk`)
/// to route traffic through the Nym mixnet.
///
/// # Future Implementations
///
/// A `SphinxRelayTransport` can be added to use custom self-hosted
/// Sphinx relay nodes (the approach used in the pre-pivot MVP).
#[async_trait]
pub trait MixnetTransport: Send + Sync {
    /// Send data through the mixnet to the configured exit service.
    ///
    /// The data is typically a JSON-RPC request destined for a Substrate
    /// full node. The transport wraps it in mixnet packets and routes
    /// it through the network.
    async fn send(&self, data: &[u8]) -> Result<()>;

    /// Receive the next message from the mixnet.
    ///
    /// Blocks until a message is available. Returns the raw response
    /// bytes (typically a JSON-RPC response from the exit service).
    async fn recv(&self) -> Result<Vec<u8>>;

    /// Get current privacy mode information.
    fn privacy_info(&self) -> PrivacyInfo;

    /// Get current transport performance metrics.
    fn metrics(&self) -> TransportMetrics;

    /// Switch privacy mode at runtime.
    ///
    /// This may involve reconnecting to the Nym network with different
    /// parameters (e.g., switching from 2-hop to 5-hop mode).
    async fn set_privacy_mode(&self, mode: PrivacyMode) -> Result<()>;

    /// Check if the transport is currently connected and operational.
    fn is_connected(&self) -> bool;

    /// Gracefully disconnect from the mixnet.
    async fn disconnect(&self) -> Result<()>;
}

/// A direct (no-mixnet) transport for baseline comparison.
///
/// Used when `PrivacyMode::None` is selected. Forwards messages
/// directly to the target Substrate full node via WebSocket.
pub struct DirectTransport {
    target_url: String,
}

impl DirectTransport {
    /// Create a new direct transport pointing at the given WebSocket URL.
    pub fn new(target_url: String) -> Self {
        Self { target_url }
    }

    /// Get the target URL.
    pub fn target_url(&self) -> &str {
        &self.target_url
    }
}
