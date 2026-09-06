---
sidebar_position: 2
title: Latency Analysis
---

# Latency Analysis

## Latency vs. Hop Count

| Hops | Forward Delay | Return Delay | Proving | Total RT |
|---|---|---|---|---|
| 1 | 500 ms | 500 ms | 2,050 ms | ~3.2 s |
| 2 | 1,000 ms | 1,000 ms | 2,100 ms | ~4.2 s |
| **3** | **1,500 ms** | **1,500 ms** | **2,150 ms** | **~5.1 s** |
| 4 | 2,000 ms | 2,000 ms | 2,200 ms | ~6.2 s |
| 5 | 2,500 ms | 2,500 ms | 2,250 ms | ~7.3 s |

## Aggregation Latency Scaling

Binary proof tree aggregation scales **logarithmically**:

| Hops | Sequential Folding | Binary Tree | Improvement |
|---|---|---|---|
| 2 | 200 ms | 100 ms | 50% |
| 3 | 300 ms | 200 ms | 33% |
| 4 | 400 ms | 200 ms | 50% |
| 5 | 500 ms | 300 ms | 40% |
| 8 | 800 ms | 300 ms | 63% |
| 16 | 1,600 ms | 400 ms | 75% |

## Optimization Strategies

### Client-Side
- **Pre-computed SURBs** — generate SURBs in background, not on critical path
- **TX validity proof caching** — reuse proof for duplicate nonce range
- **Route caching** — reuse session route for multiple packets in the same session

### Mixnode-Side
- **Concurrent proving** — base proofs generated in parallel across hops
- **Batched aggregation** — multiple packets' proof trees aggregated together
- **SIMD acceleration** — M31 field arithmetic vectorized on x86_64

### Network-Level
- **μ tuning** — lower delay parameter for latency-sensitive chains
- **Geographic routing** — prefer geographically close mixnodes (optional)
- **Reduced hops** — use 1-2 hops when latency is critical
