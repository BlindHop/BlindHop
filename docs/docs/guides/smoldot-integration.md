---
sidebar_position: 3
title: Integrating with Smoldot Apps
---

# Integrating BlindHop with Smoldot Applications

BlindHop provides a transparent privacy layer for any application using [smoldot](https://github.com/smol-dot/smoldot), the Substrate light client. This guide shows how to add mixnet privacy to your dApp with minimal code changes.

## Overview

```
Without BlindHop:
  Your dApp → smoldot → Direct WS → Full Node (sees your IP + queries)

With BlindHop (native):
  Your dApp → smoldot → WS → blindhop-proxy → Nym Mixnet → Exit → Full Node

With BlindHop (browser):
  Your dApp → BlindHop SDK → Nym Wasm → Nym Mixnet → Exit → Full Node
```

## Option 1: Native Proxy (Simplest)

The fastest way to add privacy — no code changes to your app.

### How It Works

The native `blindhop-proxy` acts as a transparent WebSocket proxy. You just change the smoldot endpoint from the full node to the local proxy:

```javascript
// BEFORE — smoldot connects directly (IP exposed)
import * as smoldot from 'smoldot';

const client = smoldot.start();
const chain = await client.addChain({
    chainSpec: kusamaChainSpec,
    // Connects to wss://rpc.polkadot.io (your IP visible)
});

// AFTER — smoldot connects through BlindHop (IP hidden)
// 1. User runs: blindhop-proxy --privacy-mode full --exit-address <addr>
// 2. Change your app's RPC endpoint:
const chain = await client.addChain({
    chainSpec: kusamaChainSpec,
    potentialRelayChains: [],
    // Override the default bootnodes to point at the local proxy
});

// Or connect via JSON-RPC over WebSocket:
const ws = new WebSocket('ws://127.0.0.1:9500');  // BlindHop proxy
```

### For Desktop/CLI Apps

```bash
# Start BlindHop proxy in the background
blindhop-proxy --privacy-mode full --exit-address <NYM_EXIT_ADDR> &

# Your app connects to ws://127.0.0.1:9500 instead of the chain directly
```

### For Electron Apps

Bundle `blindhop-proxy` as a sidecar binary and start it from your main process:

```javascript
const { spawn } = require('child_process');

const proxy = spawn('./blindhop-proxy', [
    '--privacy-mode', 'full',
    '--exit-address', exitNymAddress,
    '--listen', '127.0.0.1:9500',
]);

// Connect smoldot to the proxy
const ws = new WebSocket('ws://127.0.0.1:9500');
```

## Option 2: Browser SDK (Zero Setup for Users)

For web-based dApps where you want **zero user setup** — privacy works out of the box.

### Architecture

```javascript
import { createBlindHopClient } from '@blindhop/browser';

// Initialize BlindHop with Nym Wasm client
const blindhop = await createBlindHopClient({
    exitAddress: 'your-exit-service-nym-address',
    privacyMode: 'full',  // 'none' | 'full'
    nymApiUrl: 'https://validator.nymtech.net/api',
});

// Send JSON-RPC requests through the mixnet
const response = await blindhop.sendRpc({
    method: 'chain_getHeader',
    params: [],
});

console.log('Latest block:', parseInt(response.result.number, 16));

// Clean up
await blindhop.disconnect();
```

:::note
The `@blindhop/browser` npm package is planned for **Phase 2**. The current MVP demo implements this pattern directly. See the [demo source code](https://github.com/blindhop/blindhop/tree/main/demo) for the working implementation.
:::

### Adding a Privacy Toggle to Your UI

Give users control over their privacy level:

```javascript
// React example
function PrivacyToggle({ blindhop }) {
    const [mode, setMode] = useState('full');

    const handleChange = async (newMode) => {
        setMode(newMode);
        await blindhop.setPrivacyMode(newMode);
    };

    return (
        <div className="privacy-toggle">
            <button
                className={mode === 'none' ? 'active' : ''}
                onClick={() => handleChange('none')}
            >
                🔴 Direct
            </button>
            <button
                className={mode === 'full' ? 'active' : ''}
                onClick={() => handleChange('full')}
            >
                🟢 Private
            </button>
        </div>
    );
}
```

## Option 3: Hybrid (Auto-Detect)

The BlindHop demo uses a hybrid approach — automatically detecting whether a native proxy is running and falling back to the browser Nym client:

```javascript
async function initPrivacy() {
    // Try native proxy first (better performance, more modes)
    try {
        const ws = new WebSocket('ws://127.0.0.1:9500');
        await waitForOpen(ws, 2000);
        return { mode: 'native', transport: ws };
    } catch {
        // Fall back to browser Nym client
        const { NymBrowserClient } = await import('./nym-client.js');
        const nym = new NymBrowserClient();
        await nym.connect(EXIT_NYM_ADDRESS);
        return { mode: 'browser', transport: nym };
    }
}
```

This gives users the best of both worlds:
- **Power users** who run the native proxy get full 3-mode support
- **Casual users** get automatic browser-based privacy with no setup

## What Gets Protected

| Data | Without BlindHop | With BlindHop (Full) |
|------|:---:|:---:|
| Your IP address | Visible to full node | Hidden (Nym exit IP shown) |
| Which chain you query | Visible | Hidden |
| Query timing patterns | Visible | Obscured by cover traffic |
| Transaction submission | IP linked to extrinsic | IP hidden |
| Storage key reads | Visible (reveals interests) | Hidden |

## Limitations

- **Latency**: Full mode adds 1-3 seconds per request (Nym mixing delay)
- **Throughput**: Not suitable for high-frequency trading or sub-second queries
- **Subscriptions**: WebSocket subscriptions work but with higher latency on updates
- **Exit trust**: The exit service sees your queries (but not your IP). Run your own exit for maximum privacy

## Deploying Your Own Exit Service

For full control, run your own `blindhop-exit`:

```bash
# On a server with good connectivity
cargo build -p blindhop-exit --release
./target/release/blindhop-exit --target-rpc wss://rpc.polkadot.io

# Copy the printed Nym address and use it in your app
```

See the [deployment guide](https://wiki.blindhop.wtf/docs/guides/native-proxy) for production setup with systemd.
