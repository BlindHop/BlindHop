---
sidebar_position: 13
title: Testing & Verification
---

# Testing & Verification Guide

## Automated Test Suite

### Unit Tests

```bash
# All workspace crates: 63 tests (51 unit + 12 in common/tests/proxy_unit.rs);
# 7 live tests are #[ignore]d and skipped
cargo test --workspace

# Specific crate unit tests
cargo test -p blindhop-common
cargo test -p blindhop-proxy
cargo test -p blindhop-exit
```

### Current Test Coverage

| Crate | Tests | Coverage |
|---|---|---|
| `blindhop-common` | 23 unit + 12 integration | Config defaults, privacy mode serde, metrics percentiles, JSON-RPC round-trips, binary frame encode/decode, deflate raw compression, decompression bounds |
| `blindhop-proxy` | 11 unit | Replies routed by correlation ID, Fast/Full hop/cover-traffic/Poisson settings, WebSocket origin filtering, mode-switch replies and errors |
| `blindhop-exit` | 17 unit | Method allowlist filtering, payload size caps, concurrency limiting (permits & queueing), upstream connection pool & mock node tests |

### Integration Tests

`common/tests/proxy_unit.rs` (12 tests) is offline and runs with `cargo test --workspace`. The two live suites are `#[ignore]`d and only run on request:

```bash
# Live Nym tests (also behind the nym-live feature)
BLINDHOP_EXIT_NYM_ADDR=<address> cargo test -p blindhop-common --test nym_integration --features nym-live -- --ignored

# End-to-end tests against a running proxy
BLINDHOP_PROXY_WS=ws://127.0.0.1:9500 cargo test -p blindhop-common --test e2e_chain -- --ignored
```

| Suite | Status | Coverage |
|---|---|---|
| `proxy_unit` | ✅ Implemented | Config defaults, privacy mode properties/serde, JSON-RPC and frame round-trips, metrics snapshot/eviction/reset, control message parsing |
| `nym_integration` | 🚧 Stub (TODO) | Planned: live Nym round-trip, mixnet to Substrate, live mode switching |
| `e2e_chain` | 🚧 Stub (TODO) | Planned: client → proxy → Nym → exit → full node → SURB reply |

:::caution
The live suites are placeholders: their test bodies only print a TODO and assert nothing, so they pass without testing anything. Until they are implemented, the live path is verified manually (below).
:::

## Manual Verification

### Quick Smoke Test

1. **Start the exit service:**
   ```bash
   cargo run -p blindhop-exit -- --target-rpc wss://sys.turboflakes.io/asset-hub-paseo
   ```
   Verify: Nym address printed to stdout and saved to `.exit_nym_address`.

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
| 3 | Observe latency metrics | p50/p95 values populated (~2.0s p50) |
| 4 | Observe chart | Green line showing latency points |
| 5 | Slide to "None" | Red indicator, direct connection, lower latency |
| 6 | Slide to "Fast" | Yellow indicator, 2-hop mode (~1.4–1.8s) |
| 7 | Check overhead bar | Shows latency difference vs direct |
| 8 | Check chain data | Block number incrementing |

### Latency Benchmarking

```bash
./scripts/benchmark.sh
```

Measured latency ranges (Nym mainnet):

| Mode | Measured Round-Trip (p50) |
|---|---|
| None (direct) | 50-200 ms (direct RPC) |
| Fast (2-hop) | ~1.4–1.8 s |
| Full (5-hop) | ~2.0 s (p90 ~3.1 s) |

## CI/CD

### PR Checks (`.github/workflows/ci.yml`)

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo audit` (Rust dependency security advisories)
- `npm audit --package-lock-only --audit-level=moderate` (demo dependencies)

### Nightly (`.github/workflows/nightly.yml`)

- Full workspace release build
- `cargo test --workspace` (same tests as CI)
- `cargo test --workspace -- --ignored`: currently runs only the `e2e_chain` stubs. `nym_integration` is compiled out because the job doesn't enable `nym-live`, and no exit or proxy is started, so this step doesn't exercise the mixnet yet.
- Dependency audit (`cargo audit`, `npm audit`)

## Code Quality

```bash
# Lint check (workspace and all test/bench targets)
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check

# Fix formatting
cargo fmt --all

# Dependency security audit
cargo audit
```
