---
sidebar_position: 2
title: Browser Proxy (Wasm)
---

# Browser Proxy — Nym Wasm Client

BlindHop includes a browser-based proxy that runs the Nym mixnet client directly in your browser using WebAssembly. No local software installation needed — just open a URL.

## How It Works

```
┌─────────────────────────────────────────────────────┐
│ Browser                                             │
│  ┌──────────┐    ┌──────────────────────────────┐   │
│  │ Demo UI  │───▶│ Nym Wasm Client (Web Worker) │   │
│  │ (main)   │◀───│ @nymproject/sdk-full-fat      │   │
│  └──────────┘    └──────────┬───────────────────┘   │
└─────────────────────────────┼───────────────────────┘
                              │ Sphinx packets
                              ▼
                    ┌───────────────────┐
                    │ Nym Mixnet (500+) │
                    └────────┬──────────┘
                             ▼
                    ┌───────────────────┐
                    │ blindhop-exit     │──▶ Substrate Node
                    │ (Oracle Cloud)    │
                    └───────────────────┘
```

The `@nymproject/sdk-full-fat` npm package bundles:
- A Nym mixnet client compiled to WebAssembly
- Sphinx packet creation and parsing
- Gateway connection management
- SURB (Single Use Reply Block) handling for responses

All of this runs in a **Web Worker** to keep the main UI thread responsive.

## Auto-Detection

When you click "Start Querying", the demo automatically:

1. **Probes** `ws://localhost:9500` for a native `blindhop-proxy`
2. If found → uses native proxy (full 3-mode support)
3. If not found → starts the browser Nym Wasm client (2-mode: None/Full)

The connection mode badge in the header shows which mode is active:
- 🖥️ **Native Proxy** — blue badge
- 🌐 **Browser (Nym Wasm)** — purple badge

## Limitations vs Native Proxy

| Feature | Native Proxy | Browser Proxy |
|---------|:---:|:---:|
| None (direct) | ✅ | ✅ |
| Fast (2-hop dVPN) | ✅ | ❌ |
| Full (5-hop mixnet) | ✅ | ✅ |
| Cover traffic | ✅ | ✅ |
| Cold start time | ~2s | ~5-10s (Wasm init) |
| Memory usage | ~30 MB | ~50-80 MB |
| Works offline | After initial Nym connect | After initial Nym connect |

The Fast (2-hop) mode is not available in the browser because the Nym Wasm SDK doesn't expose low-level hop control. When in browser mode, the Fast option is greyed out with a tooltip explaining this.

## Self-Hosting the Demo

```bash
cd demo/
npm install
npm run build    # Vite builds to dist/

# Deploy dist/ to any static host:
# Vercel, Netlify, GitHub Pages, etc.
```

### Important: COOP/COEP Headers

The Nym Wasm SDK uses `SharedArrayBuffer`, which requires these HTTP headers:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

These are configured in `vercel.json` and `vite.config.js` for local dev and Vercel deployment. If self-hosting elsewhere, ensure your server sets these headers.

## Security Considerations

- **Exit address is public** — it's like a server address, not a secret
- **Your Nym client address is ephemeral** — a new address is generated each session
- **Wasm execution** — the Nym client runs sandboxed in a Web Worker
- **No local storage of keys** — keys are ephemeral and discarded on page close
- **CSP compatibility** — the `sdk-full-fat` package inlines workers and Wasm, so it works with strict CSP policies

## Performance Characteristics

| Metric | Typical Value |
|--------|---------------|
| Wasm initialization | 3-8 seconds |
| Nym gateway connection | 2-5 seconds |
| First message round-trip | 5-15 seconds (includes setup) |
| Steady-state RTT (Full mode) | 1-3 seconds |
| Browser memory usage | 50-80 MB |
| Wasm binary size | ~2-4 MB (gzipped) |
