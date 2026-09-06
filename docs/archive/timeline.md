---
sidebar_position: 15
title: Development Timeline
---

# Development Timeline (30 Weeks)

Two-developer parallel schedule with week-by-week granularity.

## Phase 1: Mixnet Core + Transaction Origin Hiding (Weeks 1–10)

**Goal:** Working BlindHop that routes extrinsic submissions through a Sphinx mixnet, hiding the sender's IP. No ZK proofs yet.

| Week | Backend (Crypto Lead) | Frontend (Integration Lead) |
|---|---|---|
| 1–2 | `blindhop-lib/sphinx`: Sphinx packet encode/decode, x25519 key exchange, AES-CTR, fixed 2 KB packets | Project scaffolding, Cargo workspace, CI/CD, linting |
| 3–4 | `blindhop-lib/sphinx`: SURB construction/processing, packet fragmentation + reassembly | `blindhop-wasm-node`: wasm-bindgen scaffolding, JS API design, TypeScript defs |
| 5–6 | `blindhop-lib/loopix`: Poisson cover traffic, exponential delay sampling, stratified cascade | `blindhop-wasm-node`: Browser WebSocket integration, platform binding |
| 7–8 | `blindhop-light-base`: `MixnetPlatform<P>` PlatformRef wrapper, `BlindHopBuilder`, `BlindHopHandle` | `blindhop-wasm-node`: demo page, privacy status indicators, npm package setup |
| 9 | `blindhop-full-node`: Sphinx relay engine, delay queue, exit node logic, `MixnodePlugin` trait | Integration testing: browser → 5 local mixnodes → local substrate-node |
| 10 | `blindhop-lib/mixnode`: Threshold engine, dual-class pool, session management | End-to-end tx submission demo, cover traffic visualization |

---

## Phase 2: ZK Relay Proofs + Metadata Privacy (Weeks 11–18)

**Goal:** Mixnodes generate Stwo proofs of correct relay. Read queries routed through mixnet. Binary proof tree operational.

| Week | Backend (Crypto Lead) | Frontend (Integration Lead) |
|---|---|---|
| 11–12 | `blindhop-zk`: Relay proof circuit (Stwo AIR constraints, Poseidon2 commitments) | `blindhop-light-base`: Extend PlatformRef for read query routing (storage proofs, headers) |
| 13–14 | `blindhop-zk/recursion`: Binary proof tree aggregation circuit, Stwo recursive verification | `blindhop-wasm-node`: Hop count controls, anonymity metrics dashboard |
| 15–16 | `blindhop-zk`: Eligibility proof circuit (validator set + registry Merkle membership) | `blindhop-light-base`: Async proof fetcher, DHT integration |
| 17–18 | `blindhop-full-node`: Integrate Stwo proving into relay engine, aggregator coordination, DHT proof storage | Full integration testing with relay proofs, performance benchmarking |

---

## Phase 3: Full ZK Suite + On-Chain Settlement (Weeks 19–26)

**Goal:** All 4 ZK proof types operational. PolkaVM contracts deployed. Cover traffic compliance enforced.

| Week | Backend (Crypto Lead) | Frontend (Integration Lead) |
|---|---|---|
| 19–20 | `blindhop-zk`: TX well-formedness proof circuit (simplified, Wasm-feasible) | `blindhop-wasm-node`: Client-side tx validity proof generation in Wasm |
| 21–22 | `blindhop-zk`: Cover traffic compliance proof circuit | `blindhop-contracts`: rEVM adapter (Solidity interface for EVM wallets) |
| 23–24 | `blindhop-contracts`: Registry contract (staking, registration, slashing) + Stwo verifier contract (Rust no_std → PVM) | Contract testing, deployment scripts, off-chain worker integration |
| 25–26 | End-to-end: on-chain verification flow, hybrid eligibility (P2P + checkpoints), session rotation testing | Documentation, API reference, developer guide, npm `@blindhop/client` publish |

---

## Phase 4: Advanced Privacy Features + Production Hardening (Weeks 27–30)

**Goal:** End-to-end privacy with pluggable value-privacy adapter. Production-ready with security audit.

| Week | Backend (Crypto Lead) | Frontend (Integration Lead) |
|---|---|---|
| 27–28 | `blindhop-light-base`: Value-privacy adapter — pluggable interface for routing confidential transaction calls through the mixnet | `blindhop-wasm-node`: Unified privacy UI — show mixnet status and privacy feature state |
| 29–30 | Integration testing: mixnet → confidential operation → verify → mixnet. Security review. | Final demo, bounty submission materials, documentation updates |
