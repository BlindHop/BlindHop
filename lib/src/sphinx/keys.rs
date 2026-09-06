//! x25519 key management and HKDF-Blake3 key derivation for Sphinx.

use rand::rngs::OsRng;
pub use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

use std::net::SocketAddr;

/// A mixnode's identity: public key + network address.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub public_key: PublicKey,
    pub address: SocketAddr,
}

/// A key pair for a relay node.
#[derive(Clone)]
pub struct RelayKeyPair {
    pub secret: StaticSecret,
    pub public: PublicKey,
}

impl RelayKeyPair {
    /// Generate a new random relay key pair.
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Reconstruct from a 32-byte secret key.
    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        let secret = StaticSecret::from(bytes);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Export the secret key as bytes.
    pub fn secret_bytes(&self) -> [u8; 32] {
        // StaticSecret doesn't expose raw bytes directly; we stored them at construction.
        // For the MVP, we serialize via the diffie-hellman identity trick.
        // In production, use a proper key storage mechanism.
        self.secret.to_bytes()
    }
}

impl std::fmt::Debug for RelayKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayKeyPair")
            .field("public", &hex::encode(self.public.as_bytes()))
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

/// Derived key material for one hop in the Sphinx route.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct HopKeys {
    /// Key for encrypting/decrypting the routing header slot.
    pub header_key: [u8; 32],
    /// Key for encrypting/decrypting the payload.
    pub payload_key: [u8; 32],
    /// Blinding factor for re-blinding the group element.
    pub blinding_factor: [u8; 32],
    /// Key for computing the MAC on the routing slot.
    pub mac_key: [u8; 32],
}

/// Perform Diffie-Hellman and derive per-hop keys using HKDF-Blake3.
pub fn compute_shared_and_derive(
    ephemeral_secret: &StaticSecret,
    recipient_public: &PublicKey,
) -> ([u8; 32], HopKeys) {
    let shared_secret = ephemeral_secret.diffie_hellman(recipient_public);
    let keys = derive_hop_keys(shared_secret.as_bytes());
    (*shared_secret.as_bytes(), keys)
}

/// Derive per-hop keys from a shared secret using Blake3 keyed hashing.
pub fn derive_hop_keys(shared_secret: &[u8; 32]) -> HopKeys {
    let header_key = blake3::derive_key("blindhop-sphinx-header-key-v1", shared_secret);
    let payload_key = blake3::derive_key("blindhop-sphinx-payload-key-v1", shared_secret);
    let blinding_factor = blake3::derive_key("blindhop-sphinx-blinding-v1", shared_secret);
    let mac_key = blake3::derive_key("blindhop-sphinx-mac-key-v1", shared_secret);

    HopKeys {
        header_key,
        payload_key,
        blinding_factor,
        mac_key,
    }
}

/// Compute a keyed MAC over data using Blake3.
pub fn compute_mac(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    blake3::keyed_hash(key, data).into()
}

/// Derive the next group element from a blinding factor.
///
/// Instead of EC scalar multiplication on the existing group element (which
/// x25519's API doesn't directly support), we use the blinding factor as a
/// fresh ephemeral secret. Both client and relay derive the same blinding
/// factor from the shared secret, so they agree on the next group element.
///
/// Client sets `current_secret = blinding_factor` and computes DH with next hop.
/// Relay sets `next_group_element = blinding_factor * G` in the forwarded header.
/// Next relay computes `DH(its_secret, blinding_factor * G)` = same shared secret.
pub fn derive_next_group_element(blinding_factor: &[u8; 32]) -> PublicKey {
    let next_secret = StaticSecret::from(*blinding_factor);
    PublicKey::from(&next_secret)
}

/// Simple hex encoding for key display/serialization.
pub mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("hex string must have even length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let kp = RelayKeyPair::generate();
        let kp2 = RelayKeyPair::generate();
        // Different keys
        assert_ne!(kp.public.as_bytes(), kp2.public.as_bytes());
    }

    #[test]
    fn test_roundtrip_secret_bytes() {
        let kp = RelayKeyPair::generate();
        let bytes = kp.secret_bytes();
        let kp2 = RelayKeyPair::from_secret_bytes(bytes);
        assert_eq!(kp.public.as_bytes(), kp2.public.as_bytes());
    }

    #[test]
    fn test_shared_secret_agreement() {
        let alice = RelayKeyPair::generate();
        let bob = RelayKeyPair::generate();

        let shared_a = alice.secret.diffie_hellman(&bob.public);
        let shared_b = bob.secret.diffie_hellman(&alice.public);
        assert_eq!(shared_a.as_bytes(), shared_b.as_bytes());
    }

    #[test]
    fn test_hop_key_derivation_deterministic() {
        let shared = [42u8; 32];
        let keys1 = derive_hop_keys(&shared);
        let keys2 = derive_hop_keys(&shared);
        assert_eq!(keys1.header_key, keys2.header_key);
        assert_eq!(keys1.payload_key, keys2.payload_key);
    }

    #[test]
    fn test_hop_keys_are_different() {
        let shared = [42u8; 32];
        let keys = derive_hop_keys(&shared);
        // All four derived keys should be different
        assert_ne!(keys.header_key, keys.payload_key);
        assert_ne!(keys.header_key, keys.blinding_factor);
        assert_ne!(keys.header_key, keys.mac_key);
    }

    #[test]
    fn test_mac_verification() {
        let key = [1u8; 32];
        let data = b"hello world";
        let mac1 = compute_mac(&key, data);
        let mac2 = compute_mac(&key, data);
        assert_eq!(mac1, mac2);

        let mac3 = compute_mac(&key, b"different data");
        assert_ne!(mac1, mac3);
    }

    #[test]
    fn test_blinding_produces_valid_key() {
        let blinding = [99u8; 32];
        let ge = derive_next_group_element(&blinding);
        // Should produce a valid non-zero public key
        assert_ne!(ge.as_bytes(), &[0u8; 32]);
        // Deterministic
        let ge2 = derive_next_group_element(&blinding);
        assert_eq!(ge.as_bytes(), ge2.as_bytes());
    }

    #[test]
    fn test_hex_roundtrip() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let encoded = hex::encode(&bytes);
        assert_eq!(encoded, "deadbeef");
        let decoded = hex::decode(&encoded).unwrap();
        assert_eq!(bytes, decoded);
    }
}
