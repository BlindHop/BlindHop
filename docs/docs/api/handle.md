---
sidebar_position: 2
title: BlindHopHandle
---

# BlindHopHandle API

The `BlindHopHandle` provides **runtime privacy controls** after the client is started.

## Methods

```rust
impl BlindHopHandle {
    /// Dynamically adjust hop count (1–5)
    pub fn set_hop_count(&self, n: u8);

    /// Adjust cover traffic rate (packets/sec)
    pub fn set_cover_rate(&self, lambda: f64);

    /// Get current anonymity metrics
    pub fn anonymity_metrics(&self) -> AnonymityMetrics;

    /// Get current privacy status
    pub fn privacy_status(&self) -> PrivacyStatus;

    /// Get latest proof status
    pub fn proof_status(&self) -> ProofStatus;
}
```

## AnonymityMetrics

```rust
pub struct AnonymityMetrics {
    pub total_mixnodes: u32,
    pub validator_mixnodes: u32,
    pub standalone_mixnodes: u32,
    pub active_layers: u8,
    pub anonymity_set_size: u32,
    pub status: PrivacyStatus,
}

pub enum PrivacyStatus {
    Active,     // Mixnet fully operational
    Degraded,   // Below threshold, BestEffort mode
    Fallback,   // Direct connection (no mixnet)
}
```

## ProofStatus

```rust
pub struct ProofStatus {
    pub latest_root_hash: Option<[u8; 32]>,
    pub verified: bool,
    pub proof_count: u64,
    pub last_verified_at: Option<Instant>,
}
```
