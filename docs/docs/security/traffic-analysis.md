---
sidebar_position: 2
title: Traffic Analysis Resistance
---

# Traffic Analysis Resistance

BlindHop employs multiple layers of defense against traffic analysis attacks.

## Defense Layers

### 1. Fixed Packet Size (2 KB)

All packets — real traffic, cover traffic, SURBs — are exactly **2,048 bytes**. An observer cannot distinguish packet types by size.

### 2. Poisson Cover Traffic

Clients and mixnodes generate cover traffic at a **constant Poisson rate**. The total traffic on any link is:

$$
\lambda_{\text{total}} = \lambda_{\text{real}} + \lambda_{\text{cover}} = \text{constant}
$$

When a user sends a real message, a cover packet is **replaced** (not added), keeping the total rate constant.

### 3. Exponential Delays

Each hop introduces a random delay sampled from Exp(μ). This:
- Reorders packets (destroys sequence correlation)
- Is memoryless (prevents timing prediction)
- Makes inter-packet intervals unpredictable

### 4. Onion Encryption

Each hop changes every bit of the packet via AES-CTR re-encryption. Packet content is completely different at each hop, preventing content-based correlation.

### 5. Group Element Re-Blinding

The Sphinx ephemeral public key is re-blinded at each hop, preventing hop-to-hop linkage of the same packet.

## Known Attack Resistance

| Attack | BlindHop Defense | Residual Risk |
|---|---|---|
| **Timing correlation** | Exponential delays | Statistical analysis over many packets |
| **Volume correlation** | Constant Poisson rate | Long-term rate changes during user sleep |
| **Packet tagging** | Onion re-encryption (every bit changes) | None — cryptographically eliminated |
| **Intersection attack** | Cover traffic from idle clients | Reduced set over very long periods |
| **n-1 attack** | Random route selection per packet | Requires controlling N-1 of N hops |
| **Epistemic attack** | Fixed 2 KB + cover | Advanced statistical methods (academic) |
