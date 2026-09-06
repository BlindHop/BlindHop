//! BlindHop Library — Sphinx/Loopix mixnet cryptographic primitives.
//!
//! This crate provides the core cryptographic building blocks for the
//! BlindHop mixnet privacy layer:
//!
//! - **Sphinx packets**: Fixed 2 KB packets with layered encryption
//! - **x25519 key exchange**: Diffie-Hellman with group element blinding
//! - **AES-256-CTR**: Layered payload encryption
//! - **Blake3**: Key derivation, MACs, and hashing
//! - **SURBs**: Single-Use Reply Blocks for anonymous return paths
//!
//! # Example
//!
//! ```rust
//! use blindhop_lib::sphinx::{SphinxPacket, RelayKeyPair, NodeInfo};
//! use blindhop_lib::sphinx::packet::prefix_payload;
//!
//! // Generate relay keys
//! let relay1 = RelayKeyPair::generate();
//! let relay2 = RelayKeyPair::generate();
//! let dest = RelayKeyPair::generate();
//!
//! // Build route
//! let route = vec![
//!     NodeInfo { public_key: relay1.public, address: "127.0.0.1:9401".parse().unwrap() },
//!     NodeInfo { public_key: relay2.public, address: "127.0.0.1:9402".parse().unwrap() },
//! ];
//! let destination = NodeInfo {
//!     public_key: dest.public,
//!     address: "127.0.0.1:8080".parse().unwrap(),
//! };
//!
//! // Create Sphinx packet
//! let payload = prefix_payload(b"hello world");
//! let (packet, keys) = SphinxPacket::create(&payload, &route, &destination).unwrap();
//! assert_eq!(packet.to_bytes().len(), 2048); // Always 2 KB
//! ```

pub mod config;
pub mod error;
pub mod sphinx;

pub use config::BlindHopConfig;
pub use error::{BlindHopError, SphinxError};
