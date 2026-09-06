//! Sphinx routing header: per-hop encrypted routing slots with MACs.
//!
//! The header contains:
//! - A group element (x25519 public key) — re-blinded at each hop
//! - N encrypted routing slots, each containing the next hop's address + public key + MAC
//! - Padding to fill unused slots (for uniform header size regardless of hop count)

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use crate::config::{HEADER_SIZE, MAX_HOPS, SLOT_SIZE};
use crate::error::SphinxError;
use crate::sphinx::keys::{self, HopKeys, NodeInfo};

use x25519_dalek::PublicKey;

/// A single routing slot in the Sphinx header.
/// Contains information for one hop in the route.
#[derive(Clone)]
pub struct RoutingSlot {
    /// Next hop's address (IPv4 + port).
    pub next_addr: SocketAddr,
    /// Next hop's public key (for the next relay to identify which shared secret to use).
    pub next_public_key: [u8; 32],
    /// MAC over this slot's contents.
    pub mac: [u8; 32],
}

/// Size breakdown: 4 (IP) + 2 (port) + 32 (pubkey) + 32 (MAC) = 70 bytes = SLOT_SIZE
const _: () = assert!(4 + 2 + 32 + 32 == SLOT_SIZE);

impl RoutingSlot {
    /// Serialize a routing slot to bytes.
    pub fn to_bytes(&self) -> [u8; SLOT_SIZE] {
        let mut buf = [0u8; SLOT_SIZE];
        match self.next_addr {
            SocketAddr::V4(addr) => {
                buf[0..4].copy_from_slice(&addr.ip().octets());
                buf[4..6].copy_from_slice(&addr.port().to_be_bytes());
            }
            SocketAddr::V6(_) => {
                // MVP: IPv4 only. For v6, we'd need larger slots.
                buf[0..4].copy_from_slice(&[0u8; 4]);
                buf[4..6].copy_from_slice(&0u16.to_be_bytes());
            }
        }
        buf[6..38].copy_from_slice(&self.next_public_key);
        buf[38..70].copy_from_slice(&self.mac);
        buf
    }

    /// Deserialize a routing slot from bytes.
    pub fn from_bytes(buf: &[u8; SLOT_SIZE]) -> Self {
        let ip = Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
        let port = u16::from_be_bytes([buf[4], buf[5]]);
        let mut next_public_key = [0u8; 32];
        next_public_key.copy_from_slice(&buf[6..38]);
        let mut mac = [0u8; 32];
        mac.copy_from_slice(&buf[38..70]);

        Self {
            next_addr: SocketAddr::V4(SocketAddrV4::new(ip, port)),
            next_public_key,
            mac,
        }
    }
}

/// The full Sphinx routing header.
#[derive(Clone)]
pub struct SphinxHeader {
    /// The ephemeral public key (group element), re-blinded at each hop.
    pub group_element: PublicKey,
    /// Raw header bytes containing encrypted routing slots.
    pub routing_data: [u8; HEADER_SIZE - 32],
}

impl SphinxHeader {
    /// Total serialized size.
    pub const SIZE: usize = HEADER_SIZE;

    /// Create a new Sphinx header for the given route.
    ///
    /// The route is ordered: [hop1, hop2, ..., hopN, destination].
    /// Each hop can only decrypt its own routing slot.
    pub fn create(
        route: &[NodeInfo],
        destination: &NodeInfo,
        hop_keys: &[HopKeys],
    ) -> Result<(Self, PublicKey), SphinxError> {
        if route.is_empty() {
            return Err(SphinxError::EmptyRoute);
        }
        if route.len() > MAX_HOPS {
            return Err(SphinxError::InvalidHopCount(route.len() as u8));
        }

        let num_hops = route.len();
        let mut routing_data = [0u8; HEADER_SIZE - 32];

        // Build routing slots in reverse order (innermost layer first).
        // The last hop's slot points to the destination.
        // Each preceding hop's slot points to the next hop.

        // Start with the innermost slot (last hop → destination)
        let mut slots: Vec<[u8; SLOT_SIZE]> = Vec::with_capacity(num_hops);

        for i in (0..num_hops).rev() {
            let slot = if i == num_hops - 1 {
                // Last hop (exit relay): deliver signal with all-zero pubkey
                RoutingSlot {
                    next_addr: destination.address,
                    next_public_key: [0u8; 32], // Deliver signal
                    mac: [0u8; 32],
                }
            } else {
                // Intermediate hops: forward to next relay
                RoutingSlot {
                    next_addr: route[i + 1].address,
                    next_public_key: *route[i + 1].public_key.as_bytes(),
                    mac: [0u8; 32],
                }
            };

            slots.push(slot.to_bytes());
        }

        // Reverse so slots[0] is for hop 0
        slots.reverse();

        // Now encrypt each slot layer-by-layer and compute MACs.
        // Slot i is encrypted with keys from hops i, i+1, ..., N-1 (outermost first).
        // But for simplicity in the MVP, each slot is encrypted only with its own hop's header key.
        // This means each hop decrypts only its own slot.

        for i in 0..num_hops {
            // Encrypt slot i with hop i's header key using AES-CTR
            let encrypted = xor_with_keystream(&slots[i], &hop_keys[i].header_key, i as u32);

            // Compute MAC over the encrypted routing data (excluding the MAC slot itself)
            let mac = keys::compute_mac(&hop_keys[i].mac_key, &encrypted[..SLOT_SIZE - 32]);

            // Write the encrypted slot + MAC into routing_data
            let offset = i * SLOT_SIZE;
            if offset + SLOT_SIZE <= routing_data.len() {
                routing_data[offset..offset + SLOT_SIZE - 32].copy_from_slice(&encrypted[..SLOT_SIZE - 32]);
                routing_data[offset + SLOT_SIZE - 32..offset + SLOT_SIZE].copy_from_slice(&mac);
            }
        }

        // Fill remaining slots with random padding
        let used = num_hops * SLOT_SIZE;
        if used < routing_data.len() {
            let mut rng = rand::thread_rng();
            use rand::RngCore;
            rng.fill_bytes(&mut routing_data[used..]);
        }

        // The initial group element is the first ephemeral public key
        // (passed in from the packet creation — we return a dummy here;
        // the real one is set in packet.rs)
        let group_element = PublicKey::from([0u8; 32]);

        Ok((
            SphinxHeader {
                group_element,
                routing_data,
            },
            group_element,
        ))
    }

    /// At a relay: decrypt the first routing slot, shift the header, return the slot.
    pub fn process_at_relay(
        &self,
        hop_keys: &HopKeys,
        hop_index: usize,
    ) -> Result<(RoutingSlot, SphinxHeader), SphinxError> {
        let offset = hop_index * SLOT_SIZE;
        if offset + SLOT_SIZE > self.routing_data.len() {
            return Err(SphinxError::DecryptionFailed);
        }

        // Extract encrypted slot
        let mut encrypted_slot = [0u8; SLOT_SIZE];
        encrypted_slot.copy_from_slice(&self.routing_data[offset..offset + SLOT_SIZE]);

        // Verify MAC
        let _expected_mac = keys::compute_mac(
            &hop_keys.mac_key,
            &encrypted_slot[..SLOT_SIZE - 32],
        );
        // The MAC is stored in the encrypted_slot at bytes [38..70] but we compute it
        // over [0..38]. Let's recompute properly.
        let slot_data = &self.routing_data[offset..offset + SLOT_SIZE - 32];
        let stored_mac = &self.routing_data[offset + SLOT_SIZE - 32..offset + SLOT_SIZE];
        let computed_mac = keys::compute_mac(&hop_keys.mac_key, slot_data);

        if computed_mac[..] != stored_mac[..] {
            return Err(SphinxError::MacVerificationFailed { hop: hop_index });
        }

        // Decrypt slot data (first 38 bytes, before MAC)
        let mut decrypted_data = [0u8; SLOT_SIZE - 32];
        decrypted_data.copy_from_slice(slot_data);
        let keystream = generate_keystream(&hop_keys.header_key, hop_index as u32, SLOT_SIZE - 32);
        for (b, k) in decrypted_data.iter_mut().zip(keystream.iter()) {
            *b ^= k;
        }

        // Parse the decrypted slot
        let mut full_slot_bytes = [0u8; SLOT_SIZE];
        full_slot_bytes[..SLOT_SIZE - 32].copy_from_slice(&decrypted_data);
        full_slot_bytes[SLOT_SIZE - 32..].copy_from_slice(stored_mac);
        let slot = RoutingSlot::from_bytes(&full_slot_bytes);

        // Create modified header for next hop (same data, just updated group element)
        let new_ge = keys::derive_next_group_element(&hop_keys.blinding_factor);
        let new_header = SphinxHeader {
            group_element: new_ge,
            routing_data: self.routing_data,
        };

        Ok((slot, new_header))
    }

    /// Serialize the header to bytes.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..32].copy_from_slice(self.group_element.as_bytes());
        buf[32..].copy_from_slice(&self.routing_data);
        buf
    }

    /// Deserialize header from bytes.
    pub fn from_bytes(buf: &[u8; HEADER_SIZE]) -> Self {
        let mut ge_bytes = [0u8; 32];
        ge_bytes.copy_from_slice(&buf[0..32]);
        let group_element = PublicKey::from(ge_bytes);

        let mut routing_data = [0u8; HEADER_SIZE - 32];
        routing_data.copy_from_slice(&buf[32..]);

        SphinxHeader {
            group_element,
            routing_data,
        }
    }
}

/// XOR data with a keystream derived from an AES key.
/// Simplified: uses Blake3 in counter mode as a keystream generator.
fn xor_with_keystream(data: &[u8], key: &[u8; 32], nonce: u32) -> Vec<u8> {
    let keystream = generate_keystream(key, nonce, data.len());
    data.iter().zip(keystream.iter()).map(|(d, k)| d ^ k).collect()
}

/// Generate a keystream of the given length using Blake3 in counter mode.
fn generate_keystream(key: &[u8; 32], nonce: u32, len: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(len);
    let mut counter = 0u32;
    while output.len() < len {
        let mut input = Vec::with_capacity(36);
        input.extend_from_slice(&nonce.to_le_bytes());
        input.extend_from_slice(&counter.to_le_bytes());
        let block = blake3::keyed_hash(key, &input);
        let bytes = block.as_bytes();
        let take = std::cmp::min(32, len - output.len());
        output.extend_from_slice(&bytes[..take]);
        counter += 1;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_slot_roundtrip() {
        let slot = RoutingSlot {
            next_addr: "127.0.0.1:9401".parse().unwrap(),
            next_public_key: [42u8; 32],
            mac: [99u8; 32],
        };
        let bytes = slot.to_bytes();
        let slot2 = RoutingSlot::from_bytes(&bytes);
        assert_eq!(slot.next_addr, slot2.next_addr);
        assert_eq!(slot.next_public_key, slot2.next_public_key);
        assert_eq!(slot.mac, slot2.mac);
    }

    #[test]
    fn test_header_serialization() {
        let ge = PublicKey::from([1u8; 32]);
        let header = SphinxHeader {
            group_element: ge,
            routing_data: [0u8; HEADER_SIZE - 32],
        };
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);
        let header2 = SphinxHeader::from_bytes(&bytes);
        assert_eq!(header.group_element.as_bytes(), header2.group_element.as_bytes());
    }

    #[test]
    fn test_keystream_deterministic() {
        let key = [5u8; 32];
        let ks1 = generate_keystream(&key, 0, 100);
        let ks2 = generate_keystream(&key, 0, 100);
        assert_eq!(ks1, ks2);

        let ks3 = generate_keystream(&key, 1, 100);
        assert_ne!(ks1, ks3);
    }
}
