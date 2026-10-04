---
sidebar_position: 2
title: Getting Started
description: Install and run BlindHop with the Nym mixnet for private Substrate light client access
---

# Getting Started

This guide walks you through running BlindHop to privately query a Substrate chain.

## Prerequisites

- **Rust** (stable, edition 2024)
- A Substrate full node or public RPC endpoint

## Build

```bash
git clone https://github.com/aspect-build/BlindHop.git
cd BlindHop
cargo build --workspace
```

## Architecture Overview

BlindHop runs as two services:

1. **`blindhop-proxy`** — runs on the user's machine, accepts WebSocket connections from smoldot, and routes traffic through the Nym mixnet
2. **`blindhop-exit`** — runs on a server, receives mixnet traffic and forwards JSON-RPC requests to a Substrate full node

```
smoldot → blindhop-proxy → Nym Mixnet → blindhop-exit → Substrate Full Node
```

## Step 1: Start the Exit Service

On a server with access to a Substrate full node:

```bash
cargo run -p blindhop-exit -- --target-rpc wss://sys.turboflakes.io/asset-hub-paseo
```

This will:
- Connect to the Nym mixnet as a Service Provider
- Generate/load client keys in `.blindhop-exit/` (persisted on disk so the exit address remains stable across restarts)
- Print the exit service's Nym address (save this!)
- Write the address to `.exit_nym_address`

*(Use `--gateway <KEY>` to pin a specific gateway, or `--data-dir <PATH>` to choose where keys are stored).*

## Step 2: Start the Proxy

On the user's machine:

```bash
cargo run -p blindhop-proxy -- \
  --listen 127.0.0.1:9500 \
  --target wss://sys.turboflakes.io/asset-hub-paseo \
  --privacy-mode full \
  --exit-address <NYM_ADDRESS_FROM_STEP_1>
```

If connecting from a web application, add `--allowed-origin <ORIGIN>` (e.g. `--allowed-origin https://demo.blindhop.wtf`).

The proxy will:
- Listen for WebSocket connections on `ws://127.0.0.1:9500`
- Refuse unlisted browser origins while allowing native smoldot connections
- Route traffic through the Nym mixnet (Full mode: 5-hop)
- Support runtime mode switching

## Step 3: Connect Your Client

Point smoldot (or any WebSocket JSON-RPC client) at the proxy:

```javascript
const ws = new WebSocket('ws://127.0.0.1:9500');

// Send standard JSON-RPC requests
ws.send(JSON.stringify({
  jsonrpc: '2.0',
  id: 1,
  method: 'chain_getHeader',
  params: [],
}));
```

## Runtime Privacy Controls

Switch privacy modes at runtime by sending a control message:

```javascript
// Switch to Fast mode (2-hop, lower latency)
ws.send(JSON.stringify({
  jsonrpc: '2.0',
  id: 99,
  method: 'blindhop_setPrivacyMode',
  params: ['fast'],
}));

// Proxy returns confirmed mode:
// {"jsonrpc":"2.0","id":99,"result":{"mode":"2-hop dVPN — IP Hidden","mode_id":"fast","status":"ok"}}

// Get current metrics
ws.send(JSON.stringify({
  jsonrpc: '2.0',
  id: 100,
  method: 'blindhop_getMetrics',
  params: [],
}));
```

## Interactive Demo

BlindHop ships with an interactive demo page with a privacy slider:

```bash
./scripts/run_demo.sh --mode full --exit-address <NYM_ADDRESS>
# Open http://localhost:8080
```

The demo provides:
- **Privacy slider** — drag between None, Fast, and Full modes
- **Live latency metrics** — p50/p95 latency, overhead vs direct
- **Real-time chart** — color-coded latency history per mode
- **Chain data** — live block numbers from the connected chain

## Configuration Reference

### Proxy CLI

| Flag | Default | Description |
|------|---------|-------------|
| `--listen` | `127.0.0.1:9500` | WebSocket listen address |
| `--target` | `wss://sys.turboflakes.io/asset-hub-paseo` | Substrate RPC endpoint URL (used in None mode) |
| `--privacy-mode` | `full` | Initial privacy mode: `none`, `fast`, `full` |
| `--exit-address` | *(required for fast/full)* | Nym address of the exit service |
| `--allowed-origin` | *(none)* | Browser origin allowed to connect (repeatable) |
| `--nym-gateway` | *(auto-select)* | Reserved: accepted but not yet used (the proxy always auto-selects a gateway) |

### Exit CLI

| Flag | Default | Description |
|------|---------|-------------|
| `--target-rpc` | `wss://sys.turboflakes.io/asset-hub-paseo` | Substrate full node RPC URL |
| `--data-dir` | `.blindhop-exit` | Directory for persistent Nym keys (keeps address stable) |
| `--gateway` | *(auto-select / kept)* | Identity key of the Nym gateway to connect through |
| `--log-level` | `info` | Log level (trace, debug, info, warn, error) |

## Next Steps

- **[Architecture Overview](/architecture/overview)** — understand the system design
- **[Crate Structure](/architecture/crate-structure)** — explore the workspace layout
- **[Testing Guide](/testing)** — run the test suite
- **[FAQ](/faq)** — common questions answered
