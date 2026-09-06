//! AES-CTR layered payload encryption for Sphinx packets.
//!
//! Each hop in the route adds one layer of AES-256-CTR encryption.
//! The client applies N layers (one per hop), and each relay peels one layer.

use aes::Aes256;
use cipher::{KeyIvInit, StreamCipher};
use ctr::Ctr64BE;

use crate::config::PAYLOAD_SIZE;

type Aes256Ctr = Ctr64BE<Aes256>;

/// Encrypt payload with one layer of AES-256-CTR.
///
/// `key`: 32-byte encryption key (derived from shared secret via HKDF).
/// `hop_index`: used as part of the IV to ensure different keystreams per hop.
pub fn encrypt_layer(data: &mut [u8], key: &[u8; 32], hop_index: u8) {
    let mut iv = [0u8; 16];
    iv[0] = hop_index;
    // Use first 8 bytes of key hash as additional IV entropy
    let iv_extra = blake3::hash(&[&key[..], &[hop_index]].concat());
    iv[1..9].copy_from_slice(&iv_extra.as_bytes()[..8]);

    let mut cipher = Aes256Ctr::new(key.into(), &iv.into());
    cipher.apply_keystream(data);
}

/// Decrypt payload by removing one layer of AES-256-CTR.
/// (AES-CTR is symmetric — encrypt and decrypt are the same operation.)
pub fn decrypt_layer(data: &mut [u8], key: &[u8; 32], hop_index: u8) {
    encrypt_layer(data, key, hop_index);
}

/// Apply multiple encryption layers to a payload (client-side).
///
/// Layers are applied in reverse order: last hop's key first, first hop's key last.
/// This way, the first relay peels the outermost layer (first hop's key).
pub fn encrypt_payload(payload: &[u8], keys: &[[u8; 32]]) -> [u8; PAYLOAD_SIZE] {
    let mut buf = [0u8; PAYLOAD_SIZE];
    let copy_len = std::cmp::min(payload.len(), PAYLOAD_SIZE);
    buf[..copy_len].copy_from_slice(&payload[..copy_len]);

    // Apply layers in reverse order (innermost first)
    for (i, key) in keys.iter().enumerate().rev() {
        encrypt_layer(&mut buf, key, i as u8);
    }
    buf
}

/// Remove one encryption layer at a relay (relay-side).
pub fn peel_layer(payload: &mut [u8; PAYLOAD_SIZE], key: &[u8; 32], hop_index: u8) {
    decrypt_layer(payload, key, hop_index);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_layer_roundtrip() {
        let key = [42u8; 32];
        let original = b"hello world";
        let mut data = original.to_vec();
        encrypt_layer(&mut data, &key, 0);
        assert_ne!(&data, original.as_slice());
        decrypt_layer(&mut data, &key, 0);
        assert_eq!(&data, original.as_slice());
    }

    #[test]
    fn test_multi_layer_roundtrip() {
        let keys = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let plaintext = b"secret message for the destination node!";

        let encrypted = encrypt_payload(plaintext, &keys);
        assert_ne!(&encrypted[..plaintext.len()], plaintext.as_slice());

        // Peel layers in forward order (hop 0, then 1, then 2)
        let mut buf = encrypted;
        for (i, key) in keys.iter().enumerate() {
            peel_layer(&mut buf, key, i as u8);
        }

        assert_eq!(&buf[..plaintext.len()], plaintext.as_slice());
    }

    #[test]
    fn test_different_keys_different_ciphertext() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let mut data1 = [0u8; 64];
        let mut data2 = [0u8; 64];
        data1.copy_from_slice(&[42u8; 64]);
        data2.copy_from_slice(&[42u8; 64]);

        encrypt_layer(&mut data1, &key1, 0);
        encrypt_layer(&mut data2, &key2, 0);
        assert_ne!(data1, data2);
    }

    #[test]
    fn test_different_hop_indices_different_ciphertext() {
        let key = [1u8; 32];
        let mut data1 = [42u8; 64];
        let mut data2 = [42u8; 64];

        encrypt_layer(&mut data1, &key, 0);
        encrypt_layer(&mut data2, &key, 1);
        assert_ne!(data1, data2);
    }

    #[test]
    fn test_payload_padding() {
        let short = b"hi";
        let keys = vec![[1u8; 32]];
        let encrypted = encrypt_payload(short, &keys);
        // Padded to full PAYLOAD_SIZE
        assert_eq!(encrypted.len(), PAYLOAD_SIZE);
    }
}
