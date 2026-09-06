//! Sphinx mixnet packet primitives.
//!
//! This module implements the Sphinx packet format for anonymous communication:
//! - **Keys**: x25519 Diffie-Hellman with HKDF-Blake3 key derivation
//! - **Header**: Per-hop encrypted routing slots with MAC verification
//! - **Payload**: Layered AES-256-CTR encryption
//! - **Packet**: Complete 2 KB Sphinx packet assembly
//! - **SURB**: Single-Use Reply Blocks for anonymous return paths

pub mod header;
pub mod keys;
pub mod packet;
pub mod payload;
pub mod surb;

// Re-export key types
pub use keys::{NodeInfo, PublicKey, RelayKeyPair};
pub use packet::{PacketKeys, RelayAction, SphinxPacket};
pub use surb::Surb;
