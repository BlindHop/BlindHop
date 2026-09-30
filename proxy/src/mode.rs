//! Privacy mode switching and active transport management.
//!
//! Manages transitions between None (direct), Fast (2-hop Nym),
//! and Full (5-hop Nym mixnet) modes at runtime.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use blindhop_common::config::{BlindHopConfig, PrivacyMode};
use blindhop_common::error::{BlindHopError, Result};
use blindhop_common::metrics::TransportMetrics;
use blindhop_common::transport::{MixnetTransport, PrivacyInfo};

use crate::nym_transport::NymTransport;

/// The currently active transport, handling mode switching.
pub enum ActiveTransport {
    /// Direct WebSocket connection (no privacy).
    Direct { target_url: String },

    /// Nym-based transport (Fast or Full mode).
    Nym { transport: Box<NymTransport> },
}

impl ActiveTransport {
    /// Create a new active transport based on the config.
    pub async fn new(config: &BlindHopConfig) -> Result<Self> {
        match config.privacy_mode {
            PrivacyMode::None => {
                tracing::info!("Starting in Direct mode (no privacy)");
                Ok(Self::Direct {
                    target_url: config.target_rpc.clone(),
                })
            }
            PrivacyMode::Fast | PrivacyMode::Full => {
                let exit_addr = config.exit_address.clone().ok_or_else(|| {
                    BlindHopError::Config(
                        "Exit address required for Nym modes. Use --exit-address.".to_string(),
                    )
                })?;

                let transport = NymTransport::connect(exit_addr, config.privacy_mode).await?;

                Ok(Self::Nym {
                    transport: Box::new(transport),
                })
            }
        }
    }

    /// Send data and receive a response through the active transport.
    pub async fn send_and_recv(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::Direct { target_url } => send_direct(data, target_url).await,
            Self::Nym { transport } => {
                transport.send(data).await?;
                transport.recv().await
            }
        }
    }

    /// Get current privacy info.
    pub fn privacy_info(&self) -> PrivacyInfo {
        match self {
            Self::Direct { .. } => PrivacyInfo {
                mode: PrivacyMode::None,
                hop_count: 0,
                cover_traffic_active: false,
                gateway_address: None,
                anonymity_set_size: None,
                our_address: None,
            },
            Self::Nym { transport } => transport.privacy_info(),
        }
    }

    /// Get current transport metrics.
    pub fn transport_metrics(&self) -> TransportMetrics {
        match self {
            Self::Direct { .. } => TransportMetrics::default(),
            Self::Nym { transport } => transport.metrics(),
        }
    }

    /// Switch to a new privacy mode.
    pub async fn switch_mode(
        &mut self,
        new_mode: PrivacyMode,
        config: &BlindHopConfig,
    ) -> Result<()> {
        let current_mode = match self {
            Self::Direct { .. } => PrivacyMode::None,
            Self::Nym { transport } => *transport.privacy_info().mode(),
        };

        if current_mode == new_mode {
            tracing::debug!("Already in {} mode", new_mode.label());
            return Ok(());
        }

        tracing::info!(
            "Switching transport: {} → {}",
            current_mode.label(),
            new_mode.label()
        );

        // Disconnect existing transport if needed
        if let Self::Nym { transport } = self {
            let _ = transport.disconnect().await;
        }

        // Create new transport for the requested mode
        match new_mode {
            PrivacyMode::None => {
                *self = Self::Direct {
                    target_url: config.target_rpc.clone(),
                };
            }
            PrivacyMode::Fast | PrivacyMode::Full => {
                let exit_addr = config.exit_address.clone().ok_or_else(|| {
                    BlindHopError::Config("Exit address required for Nym modes".to_string())
                })?;

                let transport = NymTransport::connect(exit_addr, new_mode).await?;
                *self = Self::Nym {
                    transport: Box::new(transport),
                };
            }
        }

        tracing::info!("Transport switched to {} mode", new_mode.label());
        Ok(())
    }
}

/// Helper trait to access PrivacyMode from PrivacyInfo.
trait PrivacyInfoExt {
    fn mode(&self) -> &PrivacyMode;
}

impl PrivacyInfoExt for PrivacyInfo {
    fn mode(&self) -> &PrivacyMode {
        &self.mode
    }
}

/// Send a JSON-RPC message directly to a Substrate full node via WebSocket.
///
/// Used in `PrivacyMode::None` for baseline comparison.
async fn send_direct(message: &[u8], target_url: &str) -> Result<Vec<u8>> {
    let (mut ws, _) = connect_async(target_url)
        .await
        .map_err(|e| BlindHopError::SubstrateRpc(format!("Failed to connect: {}", e)))?;

    let text = String::from_utf8_lossy(message).to_string();
    ws.send(Message::Text(text.into()))
        .await
        .map_err(|e| BlindHopError::SubstrateRpc(format!("Send failed: {}", e)))?;

    // Wait for response, skipping Ping/Pong
    loop {
        match ws.next().await {
            Some(Ok(Message::Text(t))) => return Ok(t.as_bytes().to_vec()),
            Some(Ok(Message::Binary(b))) => return Ok(b.to_vec()),
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
            Some(Ok(Message::Close(_))) => {
                return Err(BlindHopError::SubstrateRpc(
                    "Connection closed before reply".to_string(),
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
