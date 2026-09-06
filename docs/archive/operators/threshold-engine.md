---
sidebar_position: 4
title: Threshold Engine
---

# Anonymity Set Threshold Engine

The threshold engine ensures the mixnet has **enough operators** for meaningful privacy. It is **configurable per-chain**.

## Configuration

```rust
pub struct ThresholdConfig {
    /// Minimum total mixnodes required
    pub min_anonymity_set: u32,

    /// Example values per chain type:
    /// Kusama: 50
    /// Polkadot: 50
    /// Parachain: 20
    /// Solochain: 10
}
```

## Decision Logic

```mermaid
flowchart TD
    A["Count validator-mixnodes"] --> B{"≥ threshold?"}
    B -- Yes --> C["Use validators only\nStandalones deprioritized"]
    B -- No --> D["Add standalones\n(sorted by stake)"]
    D --> E{"Now ≥ threshold?"}
    E -- Yes --> F["Mixed pool active"]
    E -- No --> G{"Privacy mode?"}
    G -- Required --> H["REFUSE TO OPERATE\nEmit error"]
    G -- BestEffort --> I["Operate with WARNING\nDegraded anonymity"]
```

## Privacy Modes

| Mode | Below Threshold Behavior |
|---|---|
| `Required` | Client refuses all connections. No traffic flows. User is notified. |
| `BestEffort` | Client operates with degraded privacy. Emits `privacy-degraded` event. Cover traffic continues. |

## Why Configurable?

Different chains have different privacy requirements:
- **Kusama** (50 validators): Strong anonymity set expected
- **Solochain** (10 validators): Lower threshold acceptable
- **Dev/testing**: Can be set to 1 for local development
