---
sidebar_position: 2
title: Getting Started
description: Install and configure BlindHop for browser-based or Node.js dApps
---

# Getting Started

This guide walks you through integrating BlindHop into a dApp in under 5 minutes.

## Prerequisites

- **Node.js** ≥ 18 or modern browser with WebAssembly support
- A chain specification JSON (e.g., Kusama, Polkadot, or your parachain)

## Installation

```bash
npm install @blindhop/client
```

## Browser Usage

```typescript
import { BlindHop } from '@blindhop/client';

// Fetch your chain specification
const chainSpec = await fetch('/kusama.json').then(r => r.text());

// Start BlindHop — wraps smoldot with mixnet privacy
const client = await BlindHop.start({
  chainSpec,
  hopCount: 3,                   // 1–5 hops through the mixnet
  privacyMode: 'required',       // 'required' | 'best-effort'
  coverTrafficRate: 1.0,         // λ = 1 cover packet/sec
  delayParameter: 500.0,         // μ = 500ms mean delay per hop
  minAnonymitySet: 30,           // minimum mixnodes required
});

// Use the same JSON-RPC interface as smoldot
const blockHash = await client.sendJsonRpc(
  '{"jsonrpc":"2.0","id":1,"method":"chain_getBlockHash","params":[]}'
);

// Submit a transaction anonymously
const txResult = await client.sendJsonRpc(
  '{"jsonrpc":"2.0","id":2,"method":"author_submitExtrinsic","params":["0x..."]}'
);
```

## Node.js Usage

```typescript
import { BlindHop } from '@blindhop/client';
import { readFileSync } from 'fs';

const chainSpec = readFileSync('./kusama.json', 'utf-8');

const client = await BlindHop.start({
  chainSpec,
  hopCount: 3,
  privacyMode: 'required',
});

// Everything else is identical to browser usage
```

## Runtime Privacy Controls

Once the client is running, you can dynamically adjust privacy parameters:

```typescript
// Increase hops for higher privacy
client.setHopCount(5);

// Adjust cover traffic bandwidth
client.setCoverRate(2.0); // 2 packets/sec

// Check current privacy status
const metrics = client.anonymityMetrics();
console.log(`Active mixnodes: ${metrics.totalMixnodes}`);
console.log(`Validator mixnodes: ${metrics.validatorMixnodes}`);
console.log(`Standalone mixnodes: ${metrics.standaloneMixnodes}`);
console.log(`Privacy status: ${metrics.status}`);
// → 'active' | 'degraded' | 'fallback'

// Get latest proof status
const proof = client.proofStatus();
console.log(`Latest root proof: ${proof.rootHash}`);
console.log(`Verified: ${proof.verified}`);
```

## Privacy Modes

| Mode | Behavior |
|---|---|
| `required` | Refuse to operate if the mixnet is unavailable or below threshold. All traffic goes through the mixnet or not at all. |
| `best-effort` | Fall back to direct smoldot connection with a warning if the mixnet is degraded. Emits a `privacy-degraded` event. |

## Configuration Reference

| Parameter | Type | Default | Description |
|---|---|---|---|
| `chainSpec` | `string` | *(required)* | Chain specification JSON |
| `hopCount` | `number` | `3` | Number of mixnet hops (1–5) |
| `privacyMode` | `string` | `'required'` | Privacy failure policy |
| `coverTrafficRate` | `number` | `1.0` | Cover traffic rate (packets/sec) |
| `delayParameter` | `number` | `500.0` | Mean delay per hop (ms) |
| `minAnonymitySet` | `number` | `30` | Minimum mixnodes required |

## Interactive Demo Page

BlindHop ships with an interactive `demo.html` page for testing and visualization:

```bash
cd wasm-node && npm run dev
# Open http://localhost:8080/demo.html
```

The demo page provides:
- **Live mixnet status** — connected mixnodes, active layers, privacy status
- **Transaction submission** — send a test extrinsic through the mixnet
- **Anonymity metrics** — real-time pool composition, anonymity set size
- **Cover traffic indicators** — visualize Poisson-distributed cover packets
- **Proof verification** — view latest root proof hash and verification state
- **Hop count control** — dynamically adjust hops (1–5) and observe latency changes

## Next Steps

- **[Architecture Overview](/architecture/overview)** — understand the 3-tier system
- **[Sphinx Protocol](/protocol/sphinx)** — learn how packets are encrypted
- **[ZK Proofs](/zk/stwo-overview)** — understand the Stwo Circle STARK system
- **[Run a Mixnode](/operators/overview)** — become a mixnet operator
- **[Testing Guide](/testing)** — run automated tests and local testnet verification
