<p align="center">
  <strong>BlindHop</strong><br/>
  <em>Mixnet Privacy Layer for Smoldot Light Clients</em>
</p>

<p align="center">
  <a href="https://wiki.blindhop.wtf">Documentation</a> ·
  <a href="https://blindhop.wtf">Website</a> ·
  <a href="https://demo.blindhop.wtf">Live Demo</a>
</p>


---

## What is BlindHop?

BlindHop is a **network-layer privacy system** for Substrate-based light clients. It routes [smoldot](https://github.com/smol-dot/smoldot) traffic through the [Nym mixnet](https://nymtech.net), hiding who is submitting transactions, what they're querying, and when they're active — without any trusted setup, trusted hardware, or chain fork.

**Problem:** Substrate light clients connect directly to full nodes over libp2p, exposing the user's IP address. An ISP, network observer, or malicious full node can trivially correlate transactions to real-world identities.

**Solution:** BlindHop wraps smoldot's WebSocket traffic, routing it through the Nym mixnet:
- **500+ mix nodes** — production network, battle-tested infrastructure
- **5-hop routing** — strong unlinkability via Sphinx packet format
- **Loopix cover traffic** — real and dummy packets are indistinguishable
- **Privacy slider** — users choose between None (direct), Fast (2-hop), Full (5-hop)
- **Modular exit** — dedicated service provider or Nym SOCKS5 fallback

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│                    User Layer (Browser / CLI)                     │
│  smoldot light client → WS to blindhop-proxy                     │
│  Privacy slider: None (0-hop) | Fast (2-hop) | Full (5-hop)      │
└──────────────────────────┬────────────────────────────────────────┘
                           │ WebSocket (JSON-RPC)
┌──────────────────────────▼────────────────────────────────────────┐
│                    blindhop-proxy (Local)                          │
│  Accepts smoldot WS connections                                   │
│  None: passthrough │ Fast/Full: routes through Nym SDK            │
│  Runtime mode switching via blindhop_setPrivacyMode               │
└──────────────────────────┬────────────────────────────────────────┘
                           │ Nym Mixnet (Sphinx packets)
┌──────────────────────────▼────────────────────────────────────────┐
│                    Nym Mixnet (500+ nodes)                         │
│  Gateway → Layer 1 → Layer 2 → Layer 3 → Gateway                 │
│  Sphinx format, Poisson mixing, cover traffic                     │
└──────────────────────────┬────────────────────────────────────────┘
                           │ Nym SURB reply
┌──────────────────────────▼────────────────────────────────────────┐
│                    blindhop-exit (Service Provider)                │
│  Nym SP: receives mixnet traffic                                  │
│  Forwards JSON-RPC → Substrate full node via WS                   │
│  Returns response through Nym reply channel (SURBs)               │
└──────────────────────────┬────────────────────────────────────────┘
                           │ WebSocket
┌──────────────────────────▼────────────────────────────────────────┐
│                    Substrate Full Node                             │
│  Sees exit node IP, NOT user IP                                   │
│  Standard JSON-RPC interface, no modifications needed             │
└───────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
blindhop/
├── common/      # blindhop-common — shared types, transport traits, config
├── proxy/       # blindhop-proxy — local WS proxy + Nym client
├── exit/        # blindhop-exit — Nym service provider + Substrate forwarder
├── demo/        # Browser demo with privacy slider
├── scripts/     # run_exit.sh, run_proxy.sh, run_demo.sh, benchmark.sh
├── archive/     # Legacy Sphinx relay code (pre-Nym)
│   ├── lib/     # Original blindhop-lib (Sphinx cryptography)
│   └── relay/   # Original blindhop-relay (self-hosted nodes)
└── tests/       # Integration tests
```

## Quick Start

### Option A: Browser Demo (Zero Setup)

Open the hosted demo — no Rust, no CLI, no local setup:

1. Visit [`https://demo.blindhop.wtf`](https://demo.blindhop.wtf)
2. Paste your exit service's Nym address (or use the default)
3. Click **Start Querying**
4. Adjust the privacy slider

The browser demo uses the [Nym Wasm SDK](https://www.npmjs.com/package/@nymproject/sdk-full-fat) to run the mixnet client directly in your browser.

### Option B: Native Proxy (Full Features)

For production use or when you need Fast (2-hop) mode:

#### Prerequisites

- **Rust** (stable, edition 2024)
- A running Substrate full node or public RPC endpoint

#### Build

```bash
cargo build --workspace
```

#### Run the Exit Service

Start the exit service on a server with good connectivity:

```bash
./scripts/run_exit.sh --release
# Or directly:
cargo run -p blindhop-exit -- --target-rpc wss://sys.turboflakes.io/asset-hub-paseo
```

This prints the exit's Nym address and saves it to `.exit_nym_address`.

The exit's Nym keys live in `.blindhop-exit/` (override with `--data-dir`). Keep that directory across restarts and redeploys, and back it up: it is what keeps the exit's address stable. Deleting it gives the exit a new address, and every proxy and demo user must be reconfigured.

#### Run the Proxy

On the user's machine:

```bash
./scripts/run_proxy.sh --exit-address <EXIT_NYM_ADDRESS>
# Or directly:
cargo run -p blindhop-proxy -- \
  --privacy-mode full \
  --exit-address <EXIT_NYM_ADDRESS>
```

If `.exit_nym_address` exists (from the exit service), the proxy reads it automatically:

```bash
./scripts/run_proxy.sh   # auto-reads exit address
```

The proxy refuses connections from web pages unless their origin is allowed, so a malicious site can't take control of it (for example, switching it to direct mode). Clients that aren't browsers, such as smoldot, are unaffected. To let the hosted demo use your local proxy:

```bash
cargo run -p blindhop-proxy -- --exit-address <EXIT_NYM_ADDRESS> \
  --allowed-origin https://demo.blindhop.wtf
```

#### Run the Demo (All-in-One)

```bash
./scripts/run_demo.sh --mode full --exit-address <EXIT_NYM_ADDRESS>
```

Or use the Vite dev server after starting the proxy: `cd demo && npm run dev`

### For App Developers

Integrate BlindHop into your smoldot-based dApp:

- **Native apps**: Point smoldot at `ws://127.0.0.1:9500` while running `blindhop-proxy`
- **Browser apps**: Use the Nym Wasm SDK (see [integration guide](https://wiki.blindhop.wtf/docs/guides/smoldot-integration))
- **Hybrid**: Auto-detect native proxy, fall back to browser Nym client

See the [Guides](https://wiki.blindhop.wtf/docs/guides/native-proxy) for detailed setup.


## Privacy Modes

| Mode | Hops | Cover Traffic | Latency Overhead | IP Hidden | Metadata Private |
|------|------|--------------|------------------|-----------|-----------------|
| **None** | 0 | ✗ | 0ms | ✗ | ✗ |
| **Fast** | 2 | ✗ | ~200-500ms | ✓ | ✗ |
| **Full** | 5 | ✓ | ~1-3s | ✓ | ✓ |

Users can switch modes at runtime via the demo UI's privacy slider, or by sending a `blindhop_setPrivacyMode` JSON-RPC message to the proxy.

## Key Traits

### `MixnetTransport` (in `blindhop-common`)

```rust
#[async_trait]
pub trait MixnetTransport: Send + Sync {
    async fn send(&self, data: &[u8]) -> Result<()>;
    async fn recv(&self) -> Result<Vec<u8>>;
    fn privacy_info(&self) -> PrivacyInfo;
    fn metrics(&self) -> TransportMetrics;
    async fn set_privacy_mode(&self, mode: PrivacyMode) -> Result<()>;
    fn is_connected(&self) -> bool;
    async fn disconnect(&self) -> Result<()>;
}
```

### `ExitBackend` (in `blindhop-exit`)

```rust
#[async_trait]
pub trait ExitBackend: Send + Sync {
    async fn forward_rpc(&self, request: &[u8]) -> Result<Vec<u8>>;
    fn backend_type(&self) -> &str;
}
```

## Testing

```bash
# Unit tests (all crates, no network required)
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings
```

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `nym-sdk` | 1.21.6 | Nym mixnet client SDK |
| `tokio` | 1.x | Async runtime |
| `tokio-tungstenite` | 0.26 | WebSocket client/server |
| `async-trait` | 0.1 | Async trait support |
| `clap` | 4.x | CLI argument parsing |
| `serde` | 1.x | Serialization |

## License

Apache 2.0 + MIT dual license — same as smoldot and the Kusama/Polkadot ecosystem.
