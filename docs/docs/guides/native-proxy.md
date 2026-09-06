---
sidebar_position: 1
title: Native Proxy Setup
---

# Native Proxy Configuration Guide

The native `blindhop-proxy` is a Rust binary that runs on your machine and routes all smoldot traffic through the Nym mixnet. It provides the best performance and full privacy mode support (None / Fast 2-hop / Full 5-hop).

## When to Use Native Proxy

| Scenario | Use Native Proxy | Use Browser Proxy |
|----------|:---:|:---:|
| Production deployment | ✅ | |
| Maximum privacy (2-hop Fast mode) | ✅ | |
| Automated/scripted usage | ✅ | |
| Quick demo / evaluation | | ✅ |
| No Rust toolchain available | | ✅ |

## Installation

### Build from Source

```bash
git clone https://github.com/blindhop/blindhop.git
cd blindhop
cargo build -p blindhop-proxy --release

# Binary at: ./target/release/blindhop-proxy
```

### Verify

```bash
./target/release/blindhop-proxy --help
```

### Helper Scripts

Convenience scripts are provided in `scripts/`:

```bash
# Start exit service (builds + runs)
./scripts/run_exit.sh --release

# Start proxy (auto-reads .exit_nym_address if available)
./scripts/run_proxy.sh --mode full

# All-in-one demo (exit + proxy + demo UI)
./scripts/run_demo.sh --exit-address <NYM_ADDRESS>
```


## CLI Reference

```bash
blindhop-proxy [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--listen` | `127.0.0.1:9500` | Local WebSocket proxy address |
| `--target` | `wss://sys.turboflakes.io/asset-hub-paseo` | Substrate full node RPC URL |
| `--privacy-mode` | `full` | Privacy level: `none`, `fast`, `full` |
| `--exit-address` | (required for fast/full) | Nym address of the exit service |
| `--nym-gateway` | (auto-select) | Manual Nym gateway selection |

## Privacy Modes

### None (Direct)
```bash
blindhop-proxy --privacy-mode none --target wss://rpc.polkadot.io
```
- Direct WebSocket connection — no Nym, no privacy
- Lowest latency (0ms overhead)
- Your IP is visible to the full node

### Fast (2-hop dVPN)
```bash
blindhop-proxy --privacy-mode fast --exit-address <NYM_ADDRESS>
```
- 2-hop Nym dVPN mode
- IP hidden from full node (~200-500ms overhead)
- No cover traffic — timing analysis still possible

### Full (5-hop Mixnet)
```bash
blindhop-proxy --privacy-mode full --exit-address <NYM_ADDRESS>
```
- Full 5-hop Nym mixnet with Loopix cover traffic
- Complete metadata privacy (~1-3s overhead)
- Strongest protection against traffic analysis

## Connecting smoldot

Point your smoldot light client's WebSocket at the proxy:

```javascript
// Instead of connecting directly:
// const ws = new WebSocket('wss://rpc.polkadot.io');

// Connect through BlindHop:
const ws = new WebSocket('ws://127.0.0.1:9500');
```

All JSON-RPC traffic is automatically routed through the configured privacy mode.

## Control Messages

The proxy supports special JSON-RPC methods for runtime control:

```javascript
// Switch privacy mode at runtime
ws.send(JSON.stringify({
    jsonrpc: '2.0', id: 99,
    method: 'blindhop_setPrivacyMode',
    params: ['fast']
}));

// Get current metrics
ws.send(JSON.stringify({
    jsonrpc: '2.0', id: 100,
    method: 'blindhop_getMetrics',
    params: []
}));
```

## Running as a Service

### systemd (Linux)

```bash
sudo tee /etc/systemd/system/blindhop-proxy.service << 'EOF'
[Unit]
Description=BlindHop Proxy
After=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/blindhop-proxy \
  --privacy-mode full \
  --exit-address <NYM_ADDRESS>
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable --now blindhop-proxy
```

### launchd (macOS)

```bash
cat > ~/Library/LaunchAgents/wtf.blindhop.proxy.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>wtf.blindhop.proxy</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/blindhop-proxy</string>
        <string>--privacy-mode</string><string>full</string>
        <string>--exit-address</string><string>YOUR_NYM_ADDRESS</string>
    </array>
    <key>KeepAlive</key><true/>
</dict>
</plist>
EOF

launchctl load ~/Library/LaunchAgents/wtf.blindhop.proxy.plist
```

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `Address already in use` | Another process is on port 9500. Use `--listen 127.0.0.1:9501` |
| `Nym gateway connection failed` | Check internet connectivity. Try `--nym-gateway` with a specific gateway |
| `Exit service unreachable` | Verify the exit service is running and the Nym address is correct |
| High latency in Full mode | Normal (1-3s). Nym mixing adds deliberate delays for privacy |
