---
sidebar_position: 13
title: Testing & Verification
---

# Testing & Verification Guide

## Automated Test Suite

### Unit Tests

```bash
# Sphinx round-trips, SURB encode/decode, fragmentation, delay sampling
cargo test -p blindhop-lib

# PlatformRef wrapper, session tracking, threshold engine
cargo test -p blindhop-light-base

# Stwo circuit constraint satisfaction, proof generation/verification
cargo test -p blindhop-zk

# Relay engine, delay queue, aggregation, DHT
cargo test -p blindhop-full-node

# Contract unit tests (pallet-revive test harness)
cargo test -p blindhop-contracts

# Wasm compilation + browser test
wasm-pack test --headless --chrome blindhop-wasm-node
```

### Integration Tests

```bash
# Full end-to-end: browser → mixnet → on-chain
cargo test -p blindhop-integration --features local-testnet
```

Integration test files:

| Test File | Coverage |
|---|---|
| `sphinx_roundtrip.rs` | Packet encode → multi-hop decrypt → payload recovery |
| `mixnet_e2e.rs` | Full mixnet path: entry → N hops → exit → response via SURB |
| `zk_circuits.rs` | All 4 circuits: relay, eligibility, tx validity, cover compliance |
| `binary_tree_aggregation.rs` | Proof tree: base proofs → intermediate → root proof verification |
| `contract_integration.rs` | Registry registration, staking, slashing, verifier checkpoint flow |

## Local Testnet Verification

Step-by-step procedure for verifying BlindHop on a local development chain:

### Setup

1. **Start a local chain**
   ```bash
   substrate-node --dev
   ```

2. **Launch 5+ mixnode instances** (mixed validator + standalone, varying layers)
   ```bash
   for i in {1..5}; do
     blindhop-mixnode --dev --port $((30000 + i)) --ws-port $((9900 + i)) &
   done
   ```

3. **Deploy contracts** on local PolkaVM
   ```bash
   cargo build -p blindhop-contracts --target riscv32emac-unknown-none-polkavm
   # Deploy registry + verifier to local chain
   blindhop-deploy --dev
   ```

4. **Connect the BlindHop-wrapped light client** from browser
   ```bash
   cd wasm-node && npm run dev
   # Open http://localhost:8080/demo.html
   ```

### Verification Checklist

| # | Verification Step | Expected Result |
|---|---|---|
| 1 | Submit extrinsic through mixnet | Transaction confirmed on-chain |
| 2 | Perform storage queries | Correct responses returned through mixnet |
| 3 | Verify Stwo relay proofs at exit node | Root proof (~35 KB) verifies in < 100ms |
| 4 | Observe cover traffic | Flowing at configured λ rate, uniform 2 KB packets |
| 5 | Rotate validator set (session change) | Key refresh completes, new session keys active |
| 6 | Kill mixnodes below threshold (`Required` mode) | Client refuses all connections, emits error |
| 7 | Kill mixnodes below threshold (`BestEffort` mode) | Client warns, falls back to direct connection |
| 8 | Verify on-chain Blake3 root hash | Matches DHT-stored proof content |
| 9 | Submit fraud proof for misbehavior | Target mixnode's bond slashed |
| 10 | Register standalone operator | Appears in routing table after threshold check |
| 11 | Check proof aggregation tree | Binary tree produces valid root from base proofs |

## Manual Verification

### Browser Demo
Connect the demo page to a testnet, submit a transaction, and observe:
- Anonymity metrics (total mixnodes, pool composition, privacy status)
- Proof verification state (root hash, last verified timestamp)
- Cover traffic indicators

### Latency Testing
Time the round-trip at each hop count:

| Hops | Expected RT |
|---|---|
| 1 | ~3.2 s |
| 2 | ~4.2 s |
| 3 | ~5.1 s |
| 4 | ~6.2 s |
| 5 | ~7.3 s |

### Traffic Analysis (Wireshark)
Capture network traffic and verify:
- All packets are exactly 2,048 bytes
- Packet intervals follow Poisson distribution
- No distinguishable pattern between real and cover traffic

### Proof Size Verification
- Individual base proofs: ~8 KB
- Root proof (3-hop): ~35 KB
- On-chain commitment: exactly 128 bytes (Blake3 hash)
- PolkaVM verification time: < 10ms
