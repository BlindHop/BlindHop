---
sidebar_position: 3
title: Configuration
---

# Configuration Reference

## BlindHopConfig

```rust
pub struct BlindHopConfig {
    pub hop_count: u8,               // 1–5, default: 3
    pub cover_traffic_rate: f64,     // λ (packets/sec), default: 1.0
    pub delay_parameter: f64,        // μ (mean delay ms), default: 500.0
    pub privacy_mode: PrivacyMode,   // default: Required
    pub min_anonymity_set: u32,      // default: 30
    pub chain_type: ChainType,       // default: Kusama
}

pub enum PrivacyMode {
    /// Refuse all connections if mixnet is unavailable
    Required,
    /// Fall back to direct connection with warning
    BestEffort,
}

pub enum ChainType {
    Kusama,
    Polkadot,
    Parachain,
    Solochain,
}
```

## Recommended Configurations

| Scenario | Hops | Cover Rate | Delay | Threshold | Mode |
|---|---|---|---|---|---|
| **Maximum privacy** | 5 | 2.0 | 1000 | 50 | Required |
| **Balanced** | 3 | 1.0 | 500 | 30 | Required |
| **Low latency** | 1 | 0.5 | 200 | 10 | BestEffort |
| **Mobile** | 2 | 0.1 | 300 | 10 | BestEffort |
| **Development** | 1 | 0.0 | 10 | 1 | BestEffort |
