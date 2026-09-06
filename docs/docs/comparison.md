---
sidebar_position: 10
title: Comparison
---

# Comparison with Other Privacy Solutions

## Feature Matrix

| Feature | BlindHop (Nym) | Tor | Nym (standalone) | Substrate Mixnet Spec | Integritee |
|---|---|---|---|---|---|
| **Target** | Light clients | General | General | Full nodes | Sidechains |
| **Architecture** | Nym mixnet proxy | Onion routing | Sphinx/Loopix | Sphinx mixing | TEE enclaves |
| **Integration** | WS proxy + exit SP | SOCKS proxy | SDK | Native runtime | Sidechain bridge |
| **Anonymity set** | 500+ mix nodes | ~6000 relays | 500+ mix nodes | Chain validators | TEE operators |
| **Cover Traffic** | ✅ Loopix (via Nym) | ❌ | ✅ Loopix | ✅ | N/A |
| **Timing Resistance** | ✅ Poisson delays | ⚠️ Limited | ✅ Poisson delays | ✅ | N/A |
| **Browser Support** | ✅ (via proxy) | ⚠️ Bridge | ⚠️ Bridge | ❌ | ❌ |
| **Privacy Slider** | ✅ None/Fast/Full | ❌ | ❌ | ❌ | ❌ |
| **Substrate-native** | ✅ JSON-RPC aware | ❌ | ❌ | ✅ | ✅ |
| **Trusted Setup** | ❌ None | N/A | N/A | N/A | ✅ Intel SGX |

## Detailed Comparisons

### vs. Using Tor

Tor uses **circuit-based onion routing** without mixing or delays:
- Packets traverse in sequence through a fixed circuit (no reordering)
- No cover traffic (idle users are distinguishable from active users)
- Known vulnerable to traffic correlation by a global passive adversary

BlindHop's Nym integration provides **Loopix mixing with Poisson delays** — stronger anonymity at the cost of higher latency.

### vs. Nym Directly

Using the Nym SDK directly for Substrate access requires building your own exit service and JSON-RPC handling. BlindHop provides:
- **Ready-made exit service** (`blindhop-exit`) that understands JSON-RPC
- **Privacy slider** for runtime mode switching
- **MixnetTransport trait** for future pluggable backends
- **Substrate-aware metrics** and monitoring

### vs. Substrate Mixnet Specification

The [Substrate Mixnet Spec](https://spec.polkadot.network/#sect-mixnet) defines mixing at the **full node level**:
- Designed for full nodes, not light clients
- Requires runtime changes (pallet integration)
- No production deployment yet

BlindHop works with **existing Substrate chains** — no runtime modifications needed.

### vs. Integritee

Integritee uses **Trusted Execution Environments** (Intel SGX):
- Requires specific hardware (Intel SGX)
- Trust assumption: Intel's hardware security
- No network-level privacy (TEEs protect computation, not traffic patterns)

BlindHop provides **network-level** metadata privacy with no hardware trust assumptions.
