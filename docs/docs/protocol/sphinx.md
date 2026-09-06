---
sidebar_position: 1
title: Sphinx Packet Format
---

# Sphinx Packet Format

BlindHop uses the **Sphinx** cryptographic packet format for all mixnet traffic. Every packet is exactly **2,048 bytes** (2 KB), making real traffic indistinguishable from cover traffic.

## Packet Structure

```
┌─────────────────────────────────────────┐
│           Sphinx Packet (2048 bytes)    │
├─────────────────────────────────────────┤
│  Group Element (32 bytes)              │
│  ├── x25519 public key (ephemeral)     │
├─────────────────────────────────────────┤
│  Routing Header (512 bytes)            │
│  ├── Per-hop routing commands          │
│  ├── Encrypted with AES-CTR            │
│  ├── MAC tags per hop                  │
├─────────────────────────────────────────┤
│  Payload (1504 bytes)                  │
│  ├── Onion-encrypted data              │
│  ├── AES-CTR per layer                 │
│  ├── Padded to fixed size              │
└─────────────────────────────────────────┘
```

| Field | Size | Description |
|---|---|---|
| Group Element | 32 bytes | Ephemeral x25519 public key, re-blinded at each hop |
| Routing Header | 512 bytes | Encrypted per-hop routing info (next hop address, delay) |
| Payload | 1,504 bytes | Onion-encrypted payload (extrinsic, query, or padding) |
| **Total** | **2,048 bytes** | Fixed size for all packets |

## Key Exchange

Each hop uses **x25519 Elliptic Curve Diffie-Hellman** (via `curve25519-dalek`):

```rust
// Client constructs packet for route [N1, N2, N3]:
let ephemeral_secret = x25519::StaticSecret::random();
let ephemeral_public = x25519::PublicKey::from(&ephemeral_secret);

// Shared secret with each hop:
let s1 = ephemeral_secret.diffie_hellman(&node1_public);
let s2 = ephemeral_secret.diffie_hellman(&node2_public); // after blinding
let s3 = ephemeral_secret.diffie_hellman(&node3_public); // after blinding
```

### Group Element Re-Blinding

After each hop, the group element (ephemeral public key) is **re-blinded** using the shared secret:

$$
\alpha_{i+1} = \alpha_i \cdot h(s_i)
$$

Where:
- $\alpha_i$ is the group element at hop $i$
- $s_i$ is the shared secret at hop $i$
- $h$ is a hash-to-scalar function (Blake3)

This ensures each hop sees a **different** group element, preventing correlation.

## Routing Header

The routing header contains encrypted **routing commands** for each hop:

```rust
struct RoutingCommand {
    next_hop: SocketAddr,     // IP:port of next mixnode
    delay_seed: [u8; 16],     // Seed for deterministic delay sampling
    hop_identifier: [u8; 16], // Unique identifier for this hop
}
```

Each hop can only decrypt its own routing command. The remaining commands appear as random bytes.

### Onion Encryption

The routing header is encrypted in layers (outermost = first hop):

```
encrypt(K3, encrypt(K2, encrypt(K1, [cmd1 || cmd2 || cmd3 || padding])))
```

Each hop decrypts one layer and shifts the header left, filling the end with random padding.

## Payload Encryption

The payload is also onion-encrypted using AES-CTR with keys derived from the shared secrets:

```rust
// Encryption (client side, innermost layer first):
let key1 = kdf(s1, "payload");
let key2 = kdf(s2, "payload");
let key3 = kdf(s3, "payload");

let encrypted = aes_ctr(key3, aes_ctr(key2, aes_ctr(key1, plaintext)));

// Decryption (each hop peels one layer):
// Hop 1: aes_ctr(key1, encrypted) → still encrypted with key2, key3
// Hop 2: aes_ctr(key2, ...) → still encrypted with key3
// Hop 3: aes_ctr(key3, ...) → plaintext
```

## Fragmentation

When a message exceeds the 1,504-byte payload capacity, it is **fragmented** across multiple Sphinx packets:

```rust
struct FragmentHeader {
    message_id: [u8; 16],   // Links fragments together
    fragment_index: u16,     // Order within the message
    total_fragments: u16,    // Total fragment count
}
```

The exit node reassembles fragments before processing. Fragment headers consume 20 bytes of payload, leaving 1,484 bytes per fragment for data.

## Security Properties

| Property | Mechanism |
|---|---|
| **Bitwise unlinkability** | AES-CTR re-encryption at each hop changes every bit |
| **Sender anonymity** | Group element re-blinding prevents hop correlation |
| **Fixed packet size** | All packets are exactly 2,048 bytes |
| **Replay protection** | Hop identifiers are checked against a seen-set |
| **Integrity** | MAC tags per hop detect tampering |
