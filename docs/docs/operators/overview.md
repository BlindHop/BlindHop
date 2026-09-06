---
sidebar_position: 1
title: Operators Overview
---

# Running a BlindHop Exit Service

## Overview

BlindHop's exit service (`blindhop-exit`) runs as a Nym Service Provider, receiving anonymous traffic from the mixnet and forwarding JSON-RPC requests to Substrate full nodes.

## Requirements

- **Server** with stable internet connection
- **Rust toolchain** (stable, edition 2024)
- **Access to a Substrate full node** (direct or via public RPC endpoint)

## Quick Start

```bash
cargo build -p blindhop-exit --release

./target/release/blindhop-exit \
  --target-rpc wss://sys.turboflakes.io/asset-hub-paseo
```

The exit service will:
1. Connect to the Nym mixnet as a Service Provider
2. Print its Nym address to stdout
3. Write the address to `.exit_nym_address`
4. Begin processing incoming requests

## Architecture

The exit service has two components:
- **Nym SP loop** (`service.rs`) — receives mixnet messages, parses MixnetMessage envelopes
- **Exit backend** (`backend.rs`) — forwards JSON-RPC to the Substrate full node via WebSocket

## Future Plans

- **Community-operated exits** — registry of available exit services
- **SOCKS5 fallback** — use Nym's built-in SOCKS5 proxy
- **Multi-chain support** — route to different chains based on message metadata
- **Load balancing** — distribute across multiple full nodes
