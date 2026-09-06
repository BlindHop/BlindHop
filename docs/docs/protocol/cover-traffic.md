---
sidebar_position: 4
title: Cover Traffic
---

# Cover Traffic

Cover traffic is the backbone of BlindHop's traffic analysis resistance. Without it, an observer could infer user activity from traffic volume changes.

## Design Goal

At any point in time, every network link carries a **constant Poisson stream** of 2 KB packets. An observer monitoring any link sees the same traffic pattern whether the user is actively transacting or idle.

## Three Cover Traffic Types

### Client Loop Cover

The client generates cover packets that loop through the mixnet and return:

```
Client → Entry → Hop1 → ... → Exit → (SURB) → Client
```

**Parameters:**
- Rate: `λ_client_loop` (default: 0.5 packets/sec)
- These packets are processed identically to real traffic by all mixnodes
- The client verifies the loop completed — doubles as a liveness check

### Mixnode Drop Cover

Each mixnode generates cover packets sent to random destinations within the mixnet:

```
Mixnode_A → random route → Mixnode_B (dropped silently)
```

**Parameters:**
- Rate: `λ_node_drop` (default: 1.0 packets/sec per mixnode)
- Fills traffic volume on internal links
- Cannot be distinguished from forwarded real traffic

### Mixnode Self-Loop Cover

Each mixnode generates self-addressed loop traffic:

```
Mixnode_A → random route → Mixnode_A (verified)
```

**Parameters:**
- Rate: `λ_node_loop` (default: 0.2 packets/sec per mixnode)
- Self-monitoring for network health
- Logged for cover compliance proof generation

## Cover Compliance

Mixnodes must prove they generated the required cover traffic volume. This is enforced via a **ZK cover compliance proof** (see [Cover Compliance Proof](/zk/cover-compliance-proof)).

Non-compliant mixnodes are **slashed** — their staking bond is confiscated by the Registry contract.

## Bandwidth Considerations

| Parameter | Default | Bandwidth |
|---|---|---|
| Client loop rate | 0.5 pkt/s | ~1 KB/s upload |
| With real traffic | ~1.5 pkt/s total | ~3 KB/s upload |
| Mobile-optimized | 0.1 pkt/s | ~200 B/s upload |

The cover rate is **tunable** — lower for mobile/constrained clients, higher for desktop.
