---
sidebar_position: 10
title: Comparison
---

# Comparison with Other Privacy Solutions

## Feature Matrix

| Feature | BlindHop | Tor | Nym | Substrate Mixnet Spec | Integritee |
|---|---|---|---|---|---|
| **Target** | Light clients | General | General | Full nodes | Sidechains |
| **Architecture** | Sphinx/Loopix mixnet | Onion routing | Sphinx/Loopix | Sphinx mixing | TEE enclaves |
| **Integration** | PlatformRef wrapper | SOCKS proxy | SDK | Native runtime | Sidechain bridge |
| **ZK Proofs** | ✅ Stwo Circle STARKs | ❌ | ❌ | ❌ | ❌ |
| **Trusted Setup** | ❌ None | N/A | N/A | N/A | ✅ Intel SGX |
| **Post-Quantum** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Cover Traffic** | ✅ Loopix | ❌ | ✅ Loopix | ✅ | N/A |
| **Timing Resistance** | ✅ Exp delays | ⚠️ Limited | ✅ Exp delays | ✅ | N/A |
| **On-chain Verification** | ✅ PolkaVM | N/A | ✅ Cosmos | ❌ | ✅ (TEE) |
| **Browser Support** | ✅ Wasm | ⚠️ Bridge | ⚠️ Bridge | ❌ | ❌ |
| **Operator Model** | Validator + standalone | Volunteer | Staked | Validators | TEE operators |
| **Slashing** | ✅ ZK fraud proofs | ❌ | ✅ Economic | ❌ | ❌ |

## Detailed Comparisons

### vs. Tor

Tor uses **circuit-based onion routing** without mixing or delays. This means:
- Packets traverse in sequence through a fixed circuit (no reordering)
- No cover traffic (idle users are distinguishable from active users)
- Known vulnerable to traffic correlation by a global passive adversary

BlindHop's Loopix mixing with exponential delays provides **stronger anonymity** at the cost of higher latency.

### vs. Nym Mixnet

Nym is a general-purpose mixnet. Key differences:
- Nym is an external network (not integrated with Substrate)
- Using Nym requires a bridge/proxy (not native to smoldot)
- Nym uses its own token (NYM) for staking, not KSM/DOT
- Nym does not provide ZK proofs of correct relay

BlindHop is **natively integrated** into the Substrate ecosystem and provides **cryptographic proof** of correct behavior.

### vs. Substrate Mixnet Specification

The [Substrate Mixnet Spec](https://spec.polkadot.network/#sect-mixnet) defines mixing at the **full node level**. Key differences:
- Designed for full nodes, not light clients
- No ZK proofs of correct relay
- Requires runtime changes (pallet integration)

BlindHop extends the spec's concepts to **light clients** and adds **trustless ZK verification**.
