---
sidebar_position: 12
title: FAQ
---

# Frequently Asked Questions

## General

### What is BlindHop?
BlindHop is a mixnet privacy layer for Substrate light clients. It routes all light client traffic through the Nym mixnet (500+ nodes), hiding your IP address and transaction patterns from full nodes and network observers.

### How does BlindHop work?
BlindHop runs as a local WebSocket proxy (`blindhop-proxy`) that intercepts smoldot's JSON-RPC traffic and routes it through the Nym mixnet to an exit service (`blindhop-exit`), which forwards requests to a Substrate full node. Responses return anonymously via SURB reply paths.

### Which chains does BlindHop support?
Any chain that exposes a standard Substrate JSON-RPC endpoint — Kusama, Polkadot, Asset Hub, parachains, and solochains.

### Does BlindHop require forking smoldot?
No. BlindHop operates as a local WebSocket proxy. Smoldot connects to it like any other RPC endpoint. Zero modifications to smoldot needed.

## Privacy

### How does BlindHop differ from using a VPN?
A VPN hides your IP from the destination but the VPN provider sees all your traffic. BlindHop uses the Nym mixnet with 5 hops — **no single entity** (including any mix node or the exit service) sees both your identity and your transaction.

### What can the exit service operator see?
The exit service sees the JSON-RPC requests in plaintext (it must forward them to the full node), but it **does not know who sent them**. The SURB reply mechanism ensures anonymity of the sender.

### What about the Nym gateway?
Your entry gateway knows your IP but not your traffic content or destination. The exit gateway knows the destination but not your IP. With 5 hops between them, correlation is infeasible.

### What is the anonymity set?
In Full mode, your traffic mixes with all other Nym users (thousands). This is fundamentally different from self-hosted relays where the anonymity set is the number of your own nodes.

### What are the three privacy modes?
- **None**: Direct connection to the full node (no privacy, lowest latency)
- **Fast**: 2-hop Nym path (IP hidden, ~200-500ms overhead)
- **Full**: 5-hop Nym path with cover traffic (metadata private, ~1-3s overhead)

## Performance

### How much latency does BlindHop add?
- **None mode**: ~0ms (direct connection)
- **Fast mode**: ~200-500ms round-trip overhead
- **Full mode**: ~1-3s round-trip overhead (includes Poisson mixing delays)

### Can I switch modes at runtime?
Yes. Send a `blindhop_setPrivacyMode` JSON-RPC message to the proxy, or use the privacy slider in the demo UI.

## Operations

### How do I run the exit service?
```bash
cargo run -p blindhop-exit -- --target-rpc wss://your-substrate-node.example.com
```
The exit service connects to the Nym mixnet as a Service Provider and prints its Nym address.

### Do I need to run my own exit service?
For the MVP, yes. In the future, BlindHop will support community-operated exit services and Nym's built-in SOCKS5 proxy as a fallback.

### What are the bandwidth requirements?
The exit service handles JSON-RPC traffic (typically < 1 KB per request/response). Bandwidth is modest — similar to running a WebSocket RPC proxy.

### Can I use BlindHop without the Nym mixnet?
Yes, in **None mode**. The proxy forwards traffic directly without using Nym. This is useful for development and when privacy is not required.
