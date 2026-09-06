---
sidebar_position: 2
title: Loopix Cover Traffic
description: How Nym's Loopix protocol prevents traffic analysis
---

# Loopix Cover Traffic

In **Full privacy mode**, the Nym client generates cover traffic using the Loopix protocol. This is a key advantage over simple onion routing (like Tor).

## What Is Cover Traffic?

Cover traffic consists of dummy packets that are cryptographically indistinguishable from real traffic. They serve to:

- **Hide activity patterns**: An observer cannot tell when a user is actually sending real data
- **Prevent traffic analysis**: Constant packet flow masks bursts of real activity
- **Provide sender anonymity**: Even if all mix nodes are compromised, the observer cannot distinguish real from cover packets

## Loopix Protocol

Nym implements the Loopix anonymous communication system:

### Packet Types

| Type | Path | Purpose |
|------|------|---------|
| **Real packets** | Sender → Mix nodes → Recipient | Carry actual data |
| **Loop cover** | Sender → Mix nodes → Sender | Verify mixnet is working |
| **Drop cover** | Sender → Mix nodes → (discarded) | Add noise to traffic flow |

### Timing Distribution

All packets (real and cover) are sent according to a **Poisson distribution**:
- Packet intervals are memoryless (no timing patterns)
- Rate parameter λ controls bandwidth usage
- Real packets are inserted into the cover traffic stream

## BlindHop Privacy Modes

| Mode | Cover Traffic | Effect |
|------|--------------|--------|
| **None** | ❌ | Direct connection, no privacy |
| **Fast** | ❌ | IP hidden but timing visible |
| **Full** | ✅ | IP hidden + timing hidden + traffic analysis resistant |

## Why This Matters

Without cover traffic (e.g., Tor, VPNs), an observer can determine:
- **When** you're active (by watching packet timing)
- **How much** data you're sending (by counting packets)
- **Correlation** between your sends and blockchain events

With Loopix cover traffic, your traffic is indistinguishable from background noise.
