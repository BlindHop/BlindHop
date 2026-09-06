use serde::{Deserialize, Serialize};

/// Maximum number of hops in a Sphinx route.
pub const MAX_HOPS: usize = 5;

/// Fixed Sphinx packet size in bytes (uniform for traffic analysis resistance).
pub const PACKET_SIZE: usize = 2048;

/// Size of the routing header.
/// Each hop slot: 32 (next_pubkey) + 6 (addr: 4 IP + 2 port) + 32 (MAC) = 70 bytes
/// Plus 32 bytes for the group element. 5 slots × 70 + 32 = 382 bytes, round to 512.
pub const HEADER_SIZE: usize = 512;

/// Size of the encrypted payload.
pub const PAYLOAD_SIZE: usize = PACKET_SIZE - HEADER_SIZE;

/// Size of a routing slot in the header.
pub const SLOT_SIZE: usize = 70;

/// Size of HKDF-derived key material per hop.
pub const KEY_MATERIAL_SIZE: usize = 32;

/// BlindHop configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindHopConfig {
    /// Number of mixnet hops (1-5).
    pub hop_count: u8,
    /// Cover traffic rate (packets/sec). 0.0 = disabled.
    pub cover_traffic_rate: f64,
    /// Mean delay per hop in milliseconds.
    pub delay_parameter: f64,
    /// Privacy failure policy.
    pub privacy_mode: PrivacyMode,
    /// Minimum mixnodes required for privacy.
    pub min_anonymity_set: u32,
}

/// What happens when the mixnet is unavailable or degraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyMode {
    /// Refuse all connections if mixnet is unavailable.
    Required,
    /// Fall back to direct connection with a warning.
    BestEffort,
}

impl Default for BlindHopConfig {
    fn default() -> Self {
        Self {
            hop_count: 3,
            cover_traffic_rate: 1.0,
            delay_parameter: 500.0,
            privacy_mode: PrivacyMode::Required,
            min_anonymity_set: 30,
        }
    }
}
