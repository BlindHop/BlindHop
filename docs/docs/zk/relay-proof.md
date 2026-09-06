---
sidebar_position: 2
title: Relay Proof
---

# Relay Proof — Correct Packet Processing

The **Relay Proof** is the core ZK circuit in BlindHop. It proves that a mixnode correctly decrypted one Sphinx layer and forwarded the packet — without revealing the plaintext, the next hop's identity, or the original sender.

## Circuit Specification

| Aspect | Detail |
|---|---|
| **System** | Stwo Circle STARK (M31 field) |
| **Constraints** | ~50,000 AIR constraints |
| **Proving time** | ~50 ms (native), ~200 ms (Wasm) |
| **Proof size** | ~8 KB (base proof, before aggregation) |

## Public Inputs

| Input | Size | Description |
|---|---|---|
| `incoming_commitment` | 8 × M31 | Poseidon2 hash of the incoming 2 KB packet |
| `outgoing_commitment` | 8 × M31 | Poseidon2 hash of the outgoing 2 KB packet |
| `node_public_key` | 8 × M31 | Mixnode's x25519 public key (encoded in M31) |
| `session_index` | 1 × M31 | Current session number |

## Private Inputs (Witness)

| Input | Description |
|---|---|
| `node_secret_key` | Mixnode's x25519 private key |
| `incoming_packet` | Full 2,048-byte incoming Sphinx packet |
| `shared_secret` | x25519 ECDH shared secret |
| `routing_command` | Decrypted routing command (next hop, delay seed) |
| `outgoing_packet` | Full 2,048-byte outgoing Sphinx packet (after processing) |

## What the Circuit Proves

The relay proof circuit enforces three constraints:

### 1. Correct ECDH Key Exchange

$$
\text{shared\_secret} = \text{ECDH}(\text{node\_secret}, \text{packet\_group\_element})
$$

The mixnode correctly derived the shared secret from its private key and the packet's ephemeral public key.

### 2. Correct Layer Decryption

$$
\text{outgoing\_header} = \text{AES-CTR}(\text{key}(\text{shared\_secret}), \text{incoming\_header})[\text{shift left}]
$$

$$
\text{outgoing\_payload} = \text{AES-CTR}(\text{key}(\text{shared\_secret}), \text{incoming\_payload})
$$

The mixnode correctly peeled one onion layer (decrypted and re-encrypted).

### 3. Correct Re-Blinding

$$
\text{outgoing\_group\_element} = \text{incoming\_group\_element} \cdot h(\text{shared\_secret})
$$

The group element was correctly re-blinded for the next hop.

## Commitment Scheme

Packet commitments use **Poseidon2 over M31**:

```rust
fn commit_packet(packet: &[u8; 2048]) -> [M31; 8] {
    // Split packet into 31-bit chunks
    let elements: Vec<M31> = packet
        .chunks(4)
        .map(|chunk| M31::from_le_bytes(chunk) % M31::P)
        .collect();

    // Poseidon2 hash (sponge mode)
    poseidon2_hash_m31(&elements)
}
```

## Aggregation

Individual relay proofs are **not verified independently**. Instead, they feed into the [Binary Proof Tree](/zk/binary-proof-tree) for O(log N) recursive aggregation into a single root proof.
