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
  --target-rpc wss://sys.turboflakes.io/asset-hub-paseo \
  --data-dir .blindhop-exit
```

The exit service will:
1. Connect to the Nym mixnet as a Service Provider
2. Store cryptographic identity keys in `.blindhop-exit/` (keeping its address stable across restarts)
3. Print its Nym address to stdout and write it to `.exit_nym_address`
4. Begin processing incoming requests according to its security policy

## Production Deployment (systemd)

For running in production on a server (e.g. Ubuntu 24.04 on Oracle Cloud / Hetzner), automated scripts are provided in the [`deploy/`](https://github.com/blindhop/blindhop/tree/HEAD/deploy) directory:

```bash
# 1. Build exit binary with --locked against audited Cargo.lock
./deploy/build-exit.sh

# 2. Install systemd service, blindhop user, and start service
sudo ./deploy/install-exit.sh --target-rpc wss://sys.turboflakes.io/asset-hub-paseo

# 3. Back up cryptographic keys (archive saved to a secure location)
sudo ./deploy/backup-exit-keys.sh
```

### Hardening & Resource Limits

The provided systemd unit (`deploy/blindhop-exit.service`) applies defense-in-depth isolation:
- **System Isolation**: Runs as an unprivileged `blindhop` system user with `ProtectSystem=strict`, read-only filesystem (only `/var/lib/blindhop-exit` is writable), private `/tmp`, and no ambient capabilities.
- **Resource Limits**: Capped at `MemoryMax=320M` and 1 CPU core (`CPUQuota=100%`, `CPUWeight=50`).
- **Network & Syscalls**: Restricted to `@system-service` syscall filter and IPv4/IPv6/Unix sockets only.

### Exit Policy & Concurrency

To protect both the exit operator and the upstream Substrate full node:
- **Method Allowlist**: Forwards only single requests (with a numeric or string `id`) for an explicit list of read-only methods — chain info, blocks, read-only state (e.g. `state_getStorage`, `state_getMetadata`, `state_call`), fee queries, `system_accountNextIndex` — plus `author_submitExtrinsic`. Batches, notifications, subscriptions, node-administration/identifying methods (`author_insertKey`, `author_rotateKeys`, `system_peers`, `system_localPeerId`) and unbounded queries (`state_getKeys`, `state_getPairs`) are rejected. The full list is `ALLOWED_METHODS` in `exit/src/policy.rs`.
- **Size Limits**: Accepts requests up to 1 MiB and responses up to 4 MiB.
- **Concurrency Limiting**: Max 16 concurrent requests globally (max 8 per anonymous client tag). Requests wait up to 5s in a 32-capacity queue before being answered with `BUSY -32005`.
- **Connection Pooling**: Upstream WebSocket connections are pooled (`UpstreamPool`), matching responses by JSON-RPC ID.
- **Self-Healing Watchdog**: The exit sends itself a liveness probe through the mixnet every 60s; if no probe returns within 180s (e.g. gateway disconnect), the process exits with an error so systemd restarts it.

## Future Plans

- **Community-operated exits** — registry of available exit services
- **Multi-chain support** — route to different chains based on message metadata
- **Load balancing** — distribute across multiple full nodes
