---
sidebar_position: 14
title: Dependencies
---

# Key Dependencies

## Rust Crates

| Crate | Purpose | Version | Used By |
|---|---|---|---|
| `nym-sdk` | Nym mixnet client SDK (Sphinx, Loopix, SURB) | 1.21.6 | `blindhop-proxy`, `blindhop-exit` |
| `tokio` | Async runtime | 1.x | All crates |
| `tokio-tungstenite` | WebSocket client/server | 0.26 | `blindhop-proxy`, `blindhop-exit` |
| `async-trait` | Async trait support | 0.1 | `blindhop-common` |
| `clap` | CLI argument parsing | 4.x | `blindhop-proxy`, `blindhop-exit` |
| `serde` + `serde_json` | JSON-RPC serialization | 1.x | All crates |
| `tracing` | Structured logging | 0.1.x | `blindhop-proxy`, `blindhop-exit` |
| `anyhow` | Error handling | 1.x | `blindhop-proxy`, `blindhop-exit` |
| `thiserror` | Error derive macros | 2.x | `blindhop-common` |
| `futures-util` | Stream/sink utilities | 0.3 | `blindhop-proxy` |

## Build Tooling

| Tool | Purpose |
|---|---|
| `cargo` | Rust workspace build system |
| `rustfmt` | Code formatting |
| `clippy` | Lint checking |

## Why These Choices?

### `nym-sdk` over custom Sphinx

| Aspect | Custom Sphinx (v1) | Nym SDK (v2) |
|--------|-------------------|--------------|
| Anonymity set | 3 self-hosted nodes | 500+ production nodes |
| Maintenance | Custom crypto to maintain | Professionally maintained SDK |
| Cover traffic | Custom Loopix impl | Built-in, production-tested |
| Network effect | Zero (our nodes only) | Thousands of concurrent users |
| Audit status | Unaudited | Regularly audited |

### `tokio-tungstenite` over alternatives
- **vs. `tungstenite`**: Tokio-native async, no manual polling
- **vs. `warp`/`axum`**: Lightweight — we only need raw WebSocket, not HTTP routing

### `clap` over alternatives
- **vs. `structopt`**: Merged into clap 4.x, same derive API
- **vs. manual parsing**: Type-safe, generates help text, validation
