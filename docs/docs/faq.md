---
sidebar_position: 12
title: FAQ
---

# Frequently Asked Questions

## General

### What is BlindHop?
BlindHop is a mixnet privacy layer for the smoldot light client. It routes all light client traffic through a Sphinx/Loopix mixnet, hiding your IP address and transaction patterns.

### Does BlindHop require forking smoldot?
No. BlindHop wraps smoldot's `PlatformRef` trait — a clean abstraction boundary. No fork needed, and you automatically inherit smoldot improvements.

### Which chains does BlindHop support?
Any chain that smoldot supports — Kusama, Polkadot, parachains, and solochains. BlindHop is chain-agnostic.

### Does BlindHop add any new pallets?
No. All on-chain logic is deployed as PolkaVM smart contracts on Asset Hub. Zero new pallets.

## Privacy

### How does BlindHop differ from using a VPN?
A VPN hides your IP from the destination but the VPN provider sees all your traffic. BlindHop uses multiple hops with ZK proofs — **no single entity** (including any mixnode) sees both your identity and your transaction.

### Can a mixnode operator see my transactions?
No. Each mixnode only sees an encrypted 2 KB packet. It decrypts one layer and forwards the result, which is still encrypted for the remaining hops. Only the exit node sees the plaintext transaction, but the exit node doesn't know your IP address.

### What if the mixnet has too few nodes?
BlindHop's threshold engine detects this. In `required` mode, the client refuses to operate. In `best-effort` mode, it operates with a degraded privacy warning.

## Performance

### How much latency does BlindHop add?
With 3 hops and default settings (500ms mean delay), expect ~5 seconds round-trip. This is configurable — lower the delay parameter for faster operation.

### Can I use BlindHop on mobile?
Yes. The cover traffic rate can be lowered to ~200 bytes/sec for mobile. The TX validity proof generates in Wasm (2-3 seconds on modern phones).

### How big are the ZK proofs?
Individual base proofs are ~8 KB. The root proof (after binary tree aggregation) is ~35 KB. Only a 128-byte Blake3 hash is stored on-chain.

## Security

### Is there a trusted setup?
No. Stwo Circle STARKs are fully transparent — no trusted setup ceremony needed.

### Is BlindHop post-quantum secure?
Yes. Stwo uses hash-based commitments (Poseidon2, Blake3), which are resistant to quantum computing attacks. The x25519 key exchange is NOT post-quantum, but BlindHop can migrate to post-quantum key exchange (e.g., Kyber) when Wasm implementations mature.

### What happens if a mixnode cheats?
The ZK relay proof will be invalid, and the node's bond is slashed via a fraud proof submitted to the Registry contract.

## Operations

### How do I become a mixnode operator?
**Validators**: Run the BlindHop plugin alongside your full node. No registration needed.
**Standalone**: Register on the Registry contract with a staking bond.

### What are the bandwidth requirements?
~50 KB/s for packet relay + cover traffic on a typical mixnode. This is modest by modern standards.

### Can I use BlindHop without running a mixnode?
Yes. Regular users just install `@blindhop/client` and connect as a client. You only need to run a mixnode if you want to be an operator.
