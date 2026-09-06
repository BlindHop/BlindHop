//! Single-Use Reply Blocks (SURBs) for anonymous return paths.
//!
//! A SURB is a pre-built Sphinx header that allows the destination to send
//! a reply back to the client without knowing the client's identity or route.

use crate::config::PAYLOAD_SIZE;
use crate::error::SphinxError;
use crate::sphinx::header::SphinxHeader;
use crate::sphinx::keys::NodeInfo;
use crate::sphinx::packet::SphinxPacket;
use crate::sphinx::payload;

use rand::rngs::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

use std::net::SocketAddr;

/// A Single-Use Reply Block: pre-built return route for anonymous replies.
#[derive(Clone)]
pub struct Surb {
    /// Pre-built Sphinx header for the return path.
    pub header: SphinxHeader,
    /// Address of the first relay in the return path.
    pub first_hop: SocketAddr,
    /// Payload decryption keys (one per hop, in reverse order for client decryption).
    pub payload_keys: Vec<[u8; 32]>,
    /// Number of hops in the return path.
    pub hop_count: usize,
}

impl Surb {
    /// Create a SURB for a return path through the given relays back to the client.
    ///
    /// `return_route`: ordered relays [hop1, hop2, ..., hopN] from destination back to client.
    /// `client_node`: the client's listening address (where the reply should arrive).
    pub fn create(
        return_route: &[NodeInfo],
        client_node: &NodeInfo,
    ) -> Result<Self, SphinxError> {
        if return_route.is_empty() {
            return Err(SphinxError::EmptyRoute);
        }

        let num_hops = return_route.len();

        // Generate ephemeral key pair for the SURB
        let ephemeral_secret = StaticSecret::random_from_rng(OsRng);
        let ephemeral_public = PublicKey::from(&ephemeral_secret);

        // Derive keys for each hop
        let mut hop_keys = Vec::with_capacity(num_hops);
        let mut payload_keys = Vec::with_capacity(num_hops);
        let mut current_secret = ephemeral_secret;

        for (i, node) in return_route.iter().enumerate() {
            let (_, hk) = crate::sphinx::keys::compute_shared_and_derive(
                &current_secret,
                &node.public_key,
            );
            payload_keys.push(hk.payload_key);

            if i < num_hops - 1 {
                let next_secret = StaticSecret::from(hk.blinding_factor);
                current_secret = next_secret;
            }

            hop_keys.push(hk);
        }

        // Build the header
        let (mut header, _) = SphinxHeader::create(return_route, client_node, &hop_keys)?;
        header.group_element = ephemeral_public;

        let first_hop = return_route[0].address;

        Ok(Surb {
            header,
            first_hop,
            payload_keys,
            hop_count: num_hops,
        })
    }

    /// At the destination: wrap a reply payload in the SURB to send it back.
    pub fn wrap_reply(&self, reply_payload: &[u8]) -> Result<SphinxPacket, SphinxError> {
        if reply_payload.len() > PAYLOAD_SIZE {
            return Err(SphinxError::PayloadTooLarge {
                size: reply_payload.len(),
                max: PAYLOAD_SIZE,
            });
        }

        // Encrypt the reply with all SURB keys (in reverse, like normal Sphinx)
        let encrypted = payload::encrypt_payload(reply_payload, &self.payload_keys);

        Ok(SphinxPacket {
            header: self.header.clone(),
            payload: encrypted,
        })
    }

    /// At the client: decrypt a SURB reply that has traversed the return path.
    ///
    /// Each relay has already peeled one layer. The client peels the remaining
    /// layers to recover the original reply.
    pub fn decrypt_reply(payload: &mut [u8; PAYLOAD_SIZE], keys: &[[u8; 32]]) {
        // The reply was encrypted with keys in reverse order and each relay
        // peeled one layer in forward order. The client has all keys and
        // can decrypt whatever remains.
        for (i, key) in keys.iter().enumerate() {
            payload::peel_layer(payload, key, i as u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sphinx::keys::RelayKeyPair;
    use crate::sphinx::packet::prefix_payload;

    #[test]
    fn test_surb_creation() {
        let relay1 = RelayKeyPair::generate();
        let relay2 = RelayKeyPair::generate();
        let client_kp = RelayKeyPair::generate();

        let return_route = vec![
            NodeInfo {
                public_key: relay1.public,
                address: "127.0.0.1:9401".parse().unwrap(),
            },
            NodeInfo {
                public_key: relay2.public,
                address: "127.0.0.1:9402".parse().unwrap(),
            },
        ];
        let client_node = NodeInfo {
            public_key: client_kp.public,
            address: "127.0.0.1:7000".parse().unwrap(),
        };

        let surb = Surb::create(&return_route, &client_node).unwrap();
        assert_eq!(surb.first_hop, "127.0.0.1:9401".parse::<SocketAddr>().unwrap());
        assert_eq!(surb.payload_keys.len(), 2);
        assert_eq!(surb.hop_count, 2);
    }

    #[test]
    fn test_surb_wrap_reply() {
        let relay1 = RelayKeyPair::generate();
        let client_kp = RelayKeyPair::generate();

        let return_route = vec![NodeInfo {
            public_key: relay1.public,
            address: "127.0.0.1:9401".parse().unwrap(),
        }];
        let client_node = NodeInfo {
            public_key: client_kp.public,
            address: "127.0.0.1:7000".parse().unwrap(),
        };

        let surb = Surb::create(&return_route, &client_node).unwrap();

        let reply = prefix_payload(b"reply from destination");
        let packet = surb.wrap_reply(&reply).unwrap();
        assert_eq!(packet.to_bytes().len(), crate::config::PACKET_SIZE);
    }
}
