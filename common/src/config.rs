//! Configuration types for BlindHop.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

/// Top-level configuration for the BlindHop proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindHopConfig {
    /// Local WebSocket proxy listen address.
    pub listen_addr: SocketAddr,

    /// Target Substrate full node WebSocket URL.
    pub target_rpc: String,

    /// Privacy mode: None (direct), Fast (2-hop), Full (5-hop mixnet).
    pub privacy_mode: PrivacyMode,

    /// Exit backend type: dedicated service provider or Nym SOCKS5.
    pub exit_backend: ExitBackendType,

    /// Nym address of the BlindHop exit service (for ServiceProvider backend).
    pub exit_address: Option<String>,

    /// Optional manual Nym gateway selection. Auto-selects if None.
    pub nym_gateway: Option<String>,

    /// Browser origins allowed to connect to the local proxy (e.g.
    /// `https://demo.blindhop.wtf`). Clients that send no `Origin` header
    /// (smoldot, CLI tools) are always allowed; browsers always send one.
    #[serde(default)]
    pub allowed_origins: Vec<String>,

    /// Query interval in milliseconds for the demo.
    #[serde(default = "default_query_interval")]
    pub query_interval_ms: u64,
}

fn default_query_interval() -> u64 {
    5000
}

impl Default for BlindHopConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:9500".parse().unwrap(),
            target_rpc: "wss://sys.turboflakes.io/asset-hub-paseo".to_string(),
            privacy_mode: PrivacyMode::Full,
            exit_backend: ExitBackendType::ServiceProvider,
            exit_address: None,
            nym_gateway: None,
            allowed_origins: Vec::new(),
            query_interval_ms: default_query_interval(),
        }
    }
}

/// Privacy mode controlling the level of anonymity vs. performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyMode {
    /// Direct WebSocket connection — no privacy, lowest latency.
    /// IP is exposed to the full node.
    None,

    /// 2-hop Nym dVPN mode — IP hidden, moderate latency (~200-500ms overhead).
    /// No cover traffic, weaker traffic analysis resistance.
    Fast,

    /// 5-hop Nym mixnet mode — full metadata privacy, higher latency (~1-3s overhead).
    /// Includes Loopix cover traffic and mix delays.
    Full,
}

impl PrivacyMode {
    /// Human-readable label for the privacy mode.
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Direct — IP Exposed",
            Self::Fast => "2-hop dVPN — IP Hidden",
            Self::Full => "5-hop Mixnet — Metadata Private",
        }
    }

    /// Emoji indicator for UI display.
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::None => "🔴",
            Self::Fast => "🟡",
            Self::Full => "🟢",
        }
    }

    /// Number of hops in the Nym network for this mode.
    pub fn hop_count(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Fast => 2,
            Self::Full => 5,
        }
    }

    /// Whether cover traffic is active in this mode.
    pub fn has_cover_traffic(&self) -> bool {
        matches!(self, Self::Full)
    }
}

impl std::fmt::Display for PrivacyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Exit backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExitBackendType {
    /// Dedicated BlindHop exit service (Nym service provider).
    ServiceProvider,
    /// Nym's built-in SOCKS5 exit infrastructure.
    Socks5,
}

impl std::fmt::Display for ExitBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceProvider => write!(f, "service-provider"),
            Self::Socks5 => write!(f, "socks5"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_mode_properties() {
        assert_eq!(PrivacyMode::None.hop_count(), 0);
        assert_eq!(PrivacyMode::Fast.hop_count(), 2);
        assert_eq!(PrivacyMode::Full.hop_count(), 5);

        assert!(!PrivacyMode::None.has_cover_traffic());
        assert!(!PrivacyMode::Fast.has_cover_traffic());
        assert!(PrivacyMode::Full.has_cover_traffic());
    }

    #[test]
    fn test_default_config() {
        let config = BlindHopConfig::default();
        assert_eq!(config.privacy_mode, PrivacyMode::Full);
        assert_eq!(config.exit_backend, ExitBackendType::ServiceProvider);
        assert!(config.target_rpc.contains("paseo"));
    }

    #[test]
    fn test_privacy_mode_serde() {
        let json = serde_json::to_string(&PrivacyMode::Full).unwrap();
        assert_eq!(json, "\"full\"");

        let mode: PrivacyMode = serde_json::from_str("\"fast\"").unwrap();
        assert_eq!(mode, PrivacyMode::Fast);
    }
}
