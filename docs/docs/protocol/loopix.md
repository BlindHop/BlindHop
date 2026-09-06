---
sidebar_position: 3
title: Loopix Deep Dive
description: Detailed Loopix protocol analysis for Nym-based mixing
---

# Loopix Deep Dive

The Nym mixnet implements the [Loopix](https://arxiv.org/abs/1703.00536) anonymous communication protocol. This page covers the protocol details relevant to BlindHop.

## Stratified Cascade Topology

Nym organizes mix nodes into **layers** forming a cascade:

```mermaid
graph LR
    subgraph "Gateways (Entry)"
        G1["Gateway A"]
        G2["Gateway B"]
    end

    subgraph "Layer 1"
        M1["Mix 1A"]
        M2["Mix 1B"]
        M3["Mix 1C"]
    end

    subgraph "Layer 2"
        N1["Mix 2A"]
        N2["Mix 2B"]
        N3["Mix 2C"]
    end

    subgraph "Layer 3"
        O1["Mix 3A"]
        O2["Mix 3B"]
        O3["Mix 3C"]
    end

    subgraph "Gateways (Exit)"
        H1["Gateway C"]
        H2["Gateway D"]
    end

    G1 --> M1 & M2 & M3
    G2 --> M1 & M2 & M3
    M1 --> N1 & N2 & N3
    M2 --> N1 & N2 & N3
    M3 --> N1 & N2 & N3
    N1 --> O1 & O2 & O3
    N2 --> O1 & O2 & O3
    N3 --> O1 & O2 & O3
    O1 --> H1 & H2
    O2 --> H1 & H2
    O3 --> H1 & H2
```

The client randomly selects **one node per layer**, constructing a path like `G1 → M2 → N1 → O3 → H2`.

## Poisson Mixing

At each hop, mix nodes apply a **Poisson delay**:

- Packets are reordered (prevents timing correlation)
- The delay distribution is memoryless (observing one delay gives no information about future delays)
- Combined with cover traffic, this makes traffic analysis infeasible

## Security Properties

| Property | Guarantee |
|----------|-----------|
| **Sender anonymity** | No entity knows both sender IP and message content |
| **Receiver anonymity** | Exit service knows content but not sender |
| **Unlinkability** | Cannot link input/output packets at any hop |
| **Unobservability** | Real traffic is indistinguishable from cover traffic |

## BlindHop's Use of Loopix

BlindHop relies entirely on Nym's Loopix implementation. The `nym-sdk` manages:
- Cover traffic generation and rate control
- Mix node selection and route construction
- Sphinx packet creation and SURB management
- Gateway connection and message delivery

BlindHop adds the **application layer** on top: JSON-RPC message wrapping, privacy mode switching, and Substrate-specific exit service logic.
