---
sidebar_position: 15
title: Development Timeline
---

# Development Timeline (30 Weeks)

Updated timeline reflecting the Nym mixnet pivot.

## Phase 1: Nym Integration + Proxy/Exit MVP (Weeks 1–10) ✅

**Goal:** Working BlindHop that routes Substrate JSON-RPC through the Nym mixnet with a privacy slider.

| Week | Backend | Frontend / Integration |
|---|---|---|
| 1–2 | Project scaffolding, workspace setup, archive old Sphinx code | CI/CD pipeline, linting, dependency setup |
| 3–4 | `blindhop-common`: MixnetTransport trait, config, error types | `blindhop-common`: RPC types, MixnetMessage envelope |
| 5–6 | `blindhop-proxy`: NymTransport (nym-sdk client), mode switching | `blindhop-proxy`: WS bridge, control messages |
| 7–8 | `blindhop-exit`: Nym SP, ExitBackend trait, Substrate forwarding | Demo UI: privacy slider, latency chart, metrics |
| 9–10 | Integration testing, bug fixes, CI updates | Documentation, README, Docusaurus site updates |

**✅ Delivered:** 3-crate workspace, 63 offline tests across all crates, in-browser Nym Wasm client, upstream connection pooling, binary framing with raw deflate compression, production deployment scripts (`deploy/`), and live demo UI.

---

## Phase 2: Browser Wasm + Advanced Exit (Weeks 11–18)

**Goal:** Production hardening, SOCKS5 fallback, Nym credential support, and smoldot npm wrapper.

| Week | Backend | Frontend / Integration |
|---|---|---|
| 11–12 | SOCKS5 exit fallback investigation | smoldot PlatformRef wrapper design |
| 13–14 | Multi-exit load balancing and health checks | Latency benchmarks & stress testing |
| 15–16 | Nym credential (zk-nym) bandwidth tokens | Standalone `@blindhop/browser` SDK package |
| 17–18 | Multi-chain routing across parachains | Performance optimizations |

---

## Phase 3: ZK Proofs + On-Chain Settlement (Weeks 19–26)

**Goal:** ZK relay proofs, PolkaVM contracts, custom mixnode support.

| Week | Backend (Crypto) | Frontend (Integration) |
|---|---|---|
| 19–20 | Stwo relay proof circuit design | Contract scaffolding (PolkaVM) |
| 21–22 | Binary proof tree aggregation | Registry contract implementation |
| 23–24 | Custom mixnode `MixnetTransport` implementation | Verifier contract, slashing logic |
| 25–26 | End-to-end ZK verification flow | Contract deployment + testing |

---

## Phase 4: Production Hardening (Weeks 27–30)

**Goal:** Security audit, production deployment, final documentation.

| Week | Backend | Frontend / Integration |
|---|---|---|
| 27–28 | Security review preparation, edge case handling | Unified privacy UI, production demo |
| 29–30 | Third-party audit, fixes | Bounty submission, final docs |
