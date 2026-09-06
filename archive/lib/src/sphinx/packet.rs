//! Complete Sphinx packet: header + encrypted payload.
//!
//! A Sphinx packet is always exactly 2048 bytes (PACKET_SIZE), providing
//! uniform packet sizes for traffic analysis resistance.

use std::net::SocketAddr;

use rand::rngs::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::config::{HEADER_SIZE, MAX_HOPS, PACKET_SIZE, PAYLOAD_SIZE};
use crate::error::SphinxError;
use crate::sphinx::header::SphinxHeader;
use crate::sphinx::keys::{self, HopKeys, NodeInfo, RelayKeyPair};
use crate::sphinx::payload;

/// A complete Sphinx packet (always exactly PACKET_SIZE bytes on the wire).
#[derive(Clone)]
pub struct SphinxPacket {
    /// Routing header with encrypted per-hop slots.
    pub header: SphinxHeader,
    /// Encrypted payload (layered AES-CTR).
    pub payload: [u8; PAYLOAD_SIZE],
}

/// The result of a relay processing a Sphinx packet.
#[derive(Debug)]
pub enum RelayAction {
    /// Forward the packet to the next hop.
    Forward {
        next_addr: SocketAddr,
        packet: SphinxPacket,
    },
    /// Deliver the payload to the destination (this is the exit relay).
    Deliver {
        payload: Vec<u8>,
    },
}

/// Metadata returned alongside a created packet, used for SURB processing.
#[derive(Clone)]
pub struct PacketKeys {
    /// Per-hop payload decryption keys (needed for SURB reply decryption).
    pub payload_keys: Vec<[u8; 32]>,
    /// Number of hops.
    pub hop_count: usize,
}

impl SphinxPacket {
    /// Create a new Sphinx packet routed through the given hops to a destination.
    ///
    /// - `user_payload`: the actual data to deliver (must fit in PAYLOAD_SIZE)
    /// - `route`: ordered list of relay nodes [hop1, hop2, ..., hopN]
    /// - `destination`: the final recipient
    ///
    /// Returns the packet and the keys needed for SURB reply decryption.
    pub fn create(
        user_payload: &[u8],
        route: &[NodeInfo],
        destination: &NodeInfo,
    ) -> Result<(Self, PacketKeys), SphinxError> {
        if route.is_empty() {
            return Err(SphinxError::EmptyRoute);
        }
        if route.len() > MAX_HOPS {
            return Err(SphinxError::InvalidHopCount(route.len() as u8));
        }
        if user_payload.len() > PAYLOAD_SIZE {
            return Err(SphinxError::PayloadTooLarge {
                size: user_payload.len(),
                max: PAYLOAD_SIZE,
            });
        }

        let num_hops = route.len();

        // Generate ephemeral key pair for this packet
        let ephemeral_secret = StaticSecret::random_from_rng(OsRng);
        let ephemeral_public = PublicKey::from(&ephemeral_secret);

        // Compute shared secrets and derive keys for each hop
        let mut hop_keys: Vec<HopKeys> = Vec::with_capacity(num_hops + 1);
        let mut payload_keys: Vec<[u8; 32]> = Vec::with_capacity(num_hops + 1);
        let mut current_secret = ephemeral_secret.clone();
        let mut blinded_publics: Vec<PublicKey> = Vec::with_capacity(num_hops);
        blinded_publics.push(ephemeral_public);

        // Derive keys for each hop in the route
        for (i, node) in route.iter().enumerate() {
            let (_shared, hk) = keys::compute_shared_and_derive(&current_secret, &node.public_key);
            payload_keys.push(hk.payload_key);

            if i < num_hops - 1 {
                let next_secret = StaticSecret::from(hk.blinding_factor);
                let next_public = PublicKey::from(&next_secret);
                blinded_publics.push(next_public);
                current_secret = next_secret;
            }

            hop_keys.push(hk);
        }

        // Build the routing header (only needs relay hop keys, not destination)
        let (mut header, _) = SphinxHeader::create(route, destination, &hop_keys)?;
        header.group_element = ephemeral_public;

        // Encrypt the payload with relay layers only (exit relay delivers directly)
        let encrypted_payload = payload::encrypt_payload(user_payload, &payload_keys);

        let packet = SphinxPacket {
            header,
            payload: encrypted_payload,
        };

        let keys_meta = PacketKeys {
            payload_keys,
            hop_count: num_hops,
        };

        Ok((packet, keys_meta))
    }

    /// Process a Sphinx packet at a relay node.
    ///
    /// - `secret_key`: this relay's secret key
    /// - `hop_index`: which hop this relay is (0-indexed) — in the MVP, relays
    ///   know their position. In production, this would be inferred.
    pub fn process_at_relay(
        self,
        relay_keys: &RelayKeyPair,
        hop_index: usize,
    ) -> Result<RelayAction, SphinxError> {
        // Compute shared secret with the packet's group element
        let shared_secret = relay_keys.secret.diffie_hellman(&self.header.group_element);
        let hop_keys = keys::derive_hop_keys(shared_secret.as_bytes());

        // Process the header to get the next hop info
        let (slot, new_header) = self.header.process_at_relay(&hop_keys, hop_index)?;

        // Peel one encryption layer from the payload
        let mut new_payload = self.payload;
        payload::peel_layer(&mut new_payload, &hop_keys.payload_key, hop_index as u8);

        // Check if this is the exit relay (next_addr is the destination)
        // For now, we use a simple heuristic: if this is the last hop,
        // deliver the payload. The caller must tell us via hop_index.
        // In a real system, there'd be a flag in the routing slot.

        // Check if next_public_key is all zeros (signals "deliver")
        let is_destination = slot.next_public_key == [0u8; 32];

        if is_destination {
            // Strip padding (find actual payload length)
            let payload_vec = strip_padding(&new_payload);
            Ok(RelayAction::Deliver { payload: payload_vec })
        } else {
            let packet = SphinxPacket {
                header: new_header,
                payload: new_payload,
            };
            Ok(RelayAction::Forward {
                next_addr: slot.next_addr,
                packet,
            })
        }
    }

    /// Serialize the packet to exactly PACKET_SIZE bytes.
    pub fn to_bytes(&self) -> [u8; PACKET_SIZE] {
        let mut buf = [0u8; PACKET_SIZE];
        buf[..HEADER_SIZE].copy_from_slice(&self.header.to_bytes());
        buf[HEADER_SIZE..].copy_from_slice(&self.payload);
        buf
    }

    /// Deserialize a packet from bytes.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, SphinxError> {
        if buf.len() != PACKET_SIZE {
            return Err(SphinxError::InvalidPacketSize {
                expected: PACKET_SIZE,
                actual: buf.len(),
            });
        }

        let mut header_bytes = [0u8; HEADER_SIZE];
        header_bytes.copy_from_slice(&buf[..HEADER_SIZE]);
        let header = SphinxHeader::from_bytes(&header_bytes);

        let mut payload = [0u8; PAYLOAD_SIZE];
        payload.copy_from_slice(&buf[HEADER_SIZE..]);

        Ok(SphinxPacket { header, payload })
    }
}

/// Strip zero-padding from payload to recover original message.
/// Uses a length prefix: first 4 bytes are the actual payload length (big-endian).
fn strip_padding(padded: &[u8; PAYLOAD_SIZE]) -> Vec<u8> {
    if padded.len() < 4 {
        return Vec::new();
    }
    let len = u32::from_be_bytes([padded[0], padded[1], padded[2], padded[3]]) as usize;
    if len > PAYLOAD_SIZE - 4 {
        // Corrupted or not length-prefixed, return everything
        return padded.to_vec();
    }
    padded[4..4 + len].to_vec()
}

/// Prepend a length prefix to payload before encryption.
pub fn prefix_payload(data: &[u8]) -> Vec<u8> {
    let len = data.len() as u32;
    let mut prefixed = Vec::with_capacity(4 + data.len());
    prefixed.extend_from_slice(&len.to_be_bytes());
    prefixed.extend_from_slice(data);
    prefixed
}

impl std::fmt::Debug for SphinxPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SphinxPacket")
            .field("size", &PACKET_SIZE)
            .field("header_group_element", &keys::hex::encode(self.header.group_element.as_bytes()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_route(n: usize) -> (Vec<RelayKeyPair>, Vec<NodeInfo>, RelayKeyPair, NodeInfo) {
        let mut relay_keys = Vec::new();
        let mut route = Vec::new();
        for i in 0..n {
            let kp = RelayKeyPair::generate();
            route.push(NodeInfo {
                public_key: kp.public,
                address: format!("127.0.0.1:{}", 9401 + i).parse().unwrap(),
            });
            relay_keys.push(kp);
        }
        let dest_kp = RelayKeyPair::generate();
        let dest = NodeInfo {
            public_key: dest_kp.public,
            address: "127.0.0.1:8080".parse().unwrap(),
        };
        (relay_keys, route, dest_kp, dest)
    }

    #[test]
    fn test_packet_size() {
        let (_, route, _, dest) = make_test_route(3);
        let payload = prefix_payload(b"test message");
        let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();
        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), PACKET_SIZE, "Packet must be exactly {} bytes", PACKET_SIZE);
    }

    #[test]
    fn test_packet_serialization() {
        let (_, route, _, dest) = make_test_route(1);
        let payload = prefix_payload(b"hello");
        let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();
        let bytes = packet.to_bytes();
        let packet2 = SphinxPacket::from_bytes(&bytes).unwrap();
        assert_eq!(
            packet.header.group_element.as_bytes(),
            packet2.header.group_element.as_bytes()
        );
        assert_eq!(packet.payload, packet2.payload);
    }

    #[test]
    fn test_empty_route_fails() {
        let dest_kp = RelayKeyPair::generate();
        let dest = NodeInfo {
            public_key: dest_kp.public,
            address: "127.0.0.1:8080".parse().unwrap(),
        };
        let result = SphinxPacket::create(b"test", &[], &dest);
        assert!(result.is_err());
    }

    #[test]
    fn test_payload_too_large() {
        let (_, route, _, dest) = make_test_route(1);
        let large = vec![0u8; PAYLOAD_SIZE + 1];
        let result = SphinxPacket::create(&large, &route, &dest);
        assert!(matches!(result, Err(SphinxError::PayloadTooLarge { .. })));
    }

    #[test]
    fn test_prefix_payload_roundtrip() {
        let msg = b"hello world";
        let prefixed = prefix_payload(msg);
        assert_eq!(prefixed.len(), 4 + msg.len());

        // Simulate what strip_padding does
        let mut padded = [0u8; PAYLOAD_SIZE];
        padded[..prefixed.len()].copy_from_slice(&prefixed);
        let recovered = strip_padding(&padded);
        assert_eq!(&recovered, msg);
    }
}
