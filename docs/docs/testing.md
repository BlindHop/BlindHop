---
sidebar_position: 13
title: Testing & Verification
---

# Testing & Verification Guide

## Automated Test Suite

### Unit Tests

```bash
# Shared types, config, metrics, RPC message round-trips (14 tests)
cargo test -p blindhop-common

# All workspace crates
cargo test --workspace
```

### Current Test Coverage

| Crate | Tests | Coverage |
|---|---|---|
| `blindhop-common` | 14 | Config defaults, privacy mode serde, metrics percentiles, JSON-RPC round-trips, MixnetMessage encode/decode |
| `blindhop-proxy` | — | Compilation verified, integration tests planned |
| `blindhop-exit` | — | Compilation verified, integration tests planned |

### Integration Tests (Planned)

```bash
# Requires Nym mainnet connectivity
BLINDHOP_NYM_INTEGRATION=1 cargo test --workspace -- --ignored
```

Integration test plans:

| Test | Coverage |
|---|---|
| `proxy_direct_mode` | Proxy in None mode: WS pass-through to Substrate |
| `proxy_nym_roundtrip` | Proxy → Nym → Exit → Substrate → SURB reply |
| `mode_switching` | Runtime transition between None/Fast/Full |
| `exit_rpc_forwarding` | Exit receives MixnetMessage, forwards JSON-RPC, returns response |
| `metrics_collection` | Latency and message count tracking across modes |

## Manual Verification

### Quick Smoke Test

1. **Start the exit service:**
   ```bash
   cargo run -p blindhop-exit -- --target-rpc wss://sys.turboflakes.io/asset-hub-paseo
   ```
   Verify: Nym address printed to stdout.

2. **Start the proxy:**
   ```bash
   cargo run -p blindhop-proxy -- \
     --privacy-mode full \
     --exit-address <NYM_ADDRESS>
   ```
   Verify: "Listening on 127.0.0.1:9500" message.

3. **Query via proxy:**
   ```bash
   websocat ws://127.0.0.1:9500 <<< \
     '{"jsonrpc":"2.0","id":1,"method":"chain_getHeader"}'
   ```
   Verify: Valid JSON-RPC response with block header.

### Demo UI Verification

```bash
./scripts/run_demo.sh --mode full --exit-address <NYM_ADDRESS>
```

Open `http://localhost:8080` and verify:

| # | Verification Step | Expected Result |
|---|---|---|
| 1 | Privacy slider at "Full" | Green indicator, "5-hop Mixnet — Metadata Private" |
| 2 | Click "Start Querying" | Status badge turns green "Connected" |
| 3 | Observe latency metrics | p50/p95 values populated |
| 4 | Observe chart | Green line showing latency points |
| 5 | Slide to "None" | Red indicator, direct connection, lower latency |
| 6 | Slide to "Fast" | Yellow indicator, 2-hop mode |
| 7 | Check overhead bar | Shows latency difference vs direct |
| 8 | Check chain data | Block number incrementing |

### Latency Benchmarking

```bash
./scripts/benchmark.sh
```

Expected latency ranges:

| Mode | Expected Round-Trip |
|---|---|
| None (direct) | 50-200 ms |
| Fast (2-hop) | 200-700 ms |
| Full (5-hop) | 1-4 s |

## CI/CD

### PR Checks (`.github/workflows/ci.yml`)

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --workspace -- --check`

### Nightly (`.github/workflows/nightly.yml`)

- Full workspace release build
- Unit tests
- Nym integration tests (with `--ignored` flag)

## Code Quality

```bash
# Lint check
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --workspace -- --check

# Fix formatting
cargo fmt --workspace
```
