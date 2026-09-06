---
sidebar_position: 1
title: Performance Benchmarks
---

# Performance Benchmarks

Target performance metrics for BlindHop across different environments.

## ZK Proving Performance

| Circuit | Constraints | Native (x86_64) | Wasm (Browser) | Proof Size |
|---|---|---|---|---|
| **Relay Proof** | ~50K | ~50 ms | ~200 ms | ~8 KB |
| **Eligibility Proof** | ~30K | ~30 ms | ~120 ms | ~6 KB |
| **TX Validity Proof** | ~15K | ~200 ms | ~2,000 ms | ~4 KB |
| **Cover Compliance** | ~10K | ~20 ms | ~80 ms | ~3 KB |
| **Aggregation (pair)** | ~100K | ~100 ms | N/A | ~12 KB |
| **Root Proof (3-hop)** | ~200K | ~150 ms | N/A | ~35 KB |

## Verification Performance

| Environment | Verification Time | Notes |
|---|---|---|
| **Native (x86_64)** | < 10 ms | Reference implementation |
| **PolkaVM (RISC-V JIT)** | < 10 ms | ~80% bare-metal speed |
| **Wasm (Browser)** | < 100 ms | For client-side verification |
| **Off-chain worker** | < 15 ms | In Substrate runtime |

## Latency Budget (3-Hop Path)

| Phase | Duration | Cumulative |
|---|---|---|
| TX validity proof (Wasm) | 2,000 ms | 2,000 ms |
| Sphinx packet construction | 5 ms | 2,005 ms |
| Hop 1 (delay + relay) | 500 ms | 2,505 ms |
| Hop 2 (delay + relay) | 500 ms | 3,005 ms |
| Hop 3 (delay + relay) | 500 ms | 3,505 ms |
| Exit → RPC → Result | 100 ms | 3,605 ms |
| SURB return (3 hops) | 1,500 ms | 5,105 ms |
| **Total round-trip** | **~5.1 seconds** | |

## Throughput

| Metric | Value | Notes |
|---|---|---|
| Packets per second per mixnode | ~1,000 | Sphinx relay + proof gen |
| Cover traffic per client | ~1 KB/s | λ = 0.5 pkt/s |
| DHT proof storage | ~35 KB per batch | One root proof per proof batch |
| On-chain footprint per batch | 128 bytes | Blake3 root hash only |

## Comparison to Alternatives

| System | Round-trip Latency | Proof Overhead | On-chain Cost |
|---|---|---|---|
| **Direct smoldot** | ~200 ms | None | None |
| **BlindHop (3 hops)** | ~5,100 ms | ~35 KB proof | 128 bytes |
| **Tor + smoldot** | ~2,000 ms | None | None |
| **Nym + custom** | ~3,000 ms | None | Nym staking |
