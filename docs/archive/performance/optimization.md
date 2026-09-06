---
sidebar_position: 3
title: Latency Optimization
---

# Latency Optimization Strategies

## Client-Side Optimizations

### Pre-computed SURBs
Generate SURBs in a background task, not on the critical path. The client maintains a pool of ready-to-use SURBs:

```rust
// Background task pre-generates SURBs
let surb_pool = SurbPool::new(pool_size: 10);
// On send, grab a pre-built SURB instantly
let surb = surb_pool.take().await;
```

### TX Validity Proof Caching
Reuse proofs for transactions sharing the same nonce range. Avoid regenerating the ~2s Wasm proof when submitting batched transactions.

### Route Caching
Reuse the session route for multiple packets within the same session, avoiding repeated route computation.

## Mixnode-Side Optimizations

### Concurrent Proving
Base proofs are generated in parallel across hops — each mixnode proves independently, then aggregation happens in the binary tree.

### Batched Aggregation
Multiple packets' proof trees can be aggregated together, amortizing the per-packet proof overhead.

### SIMD Acceleration
M31 field arithmetic is vectorized on x86_64 using AVX2/AVX-512 intrinsics, giving ~4x speedup for Stwo proving.

## Network-Level Optimizations

### μ Tuning
Lower the delay parameter for latency-sensitive chains. The tradeoff is reduced mixing quality.

### Geographic Routing (Optional)
Prefer geographically close mixnodes to reduce network round-trip time. This is configurable and disabled by default (prioritizes anonymity set size over latency).

### Reduced Hops
Use 1–2 hops when latency is critical and the threat model permits weaker anonymity.
