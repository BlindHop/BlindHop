---
sidebar_position: 1
title: BlindHopBuilder
---

# BlindHopBuilder API

The `BlindHopBuilder` is the primary entry point for configuring and starting a BlindHop-wrapped smoldot client.

## Rust API

```rust
use blindhop_light_base::{BlindHopBuilder, PrivacyMode};

let (client, handle) = BlindHopBuilder::new()
    .hop_count(3)
    .privacy_mode(PrivacyMode::Required)
    .cover_traffic_rate(1.0)
    .delay_parameter(500.0)
    .min_anonymity_set(30)
    .build(platform, chain_spec)
    .await?;
```

## Builder Methods

| Method | Type | Default | Description |
|---|---|---|---|
| `hop_count(n)` | `u8` | `3` | Number of mixnet hops (1–5) |
| `privacy_mode(mode)` | `PrivacyMode` | `Required` | Failure policy |
| `cover_traffic_rate(λ)` | `f64` | `1.0` | Cover packets per second |
| `delay_parameter(μ)` | `f64` | `500.0` | Mean delay per hop (ms) |
| `min_anonymity_set(n)` | `u32` | `30` | Minimum mixnodes required |
| `chain_type(t)` | `ChainType` | `Kusama` | Chain context for defaults |
| `build(platform, spec)` | async | — | Start the wrapped client |

## Return Values

`build()` returns a tuple:
- **`Client`** — standard smoldot `Client` with the same JSON-RPC interface
- **`BlindHopHandle`** — runtime privacy controls (see [Handle API](/api/handle))
