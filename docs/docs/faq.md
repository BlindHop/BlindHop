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
- **Fast**: 2-hop Nym path between gateways, skipping mix nodes (IP hidden, ~1.4–1.8 s measured round-trip)
- **Full**: 5-hop Nym path with Loopix cover traffic (metadata private, ~2.0 s round-trip p50, ~3.1 s p90)

## Performance

### How much latency does BlindHop add?
- **None mode**: ~0ms proxy overhead (direct RPC connection)
- **Fast mode**: ~1.4–1.8 s round-trip (bypasses 3 mix layers and disables cover traffic)
- **Full mode**: ~2.0 s round-trip (p90 ~3.1 s, includes Loopix cover traffic and Poisson mixing delays)

Large chain payloads like `state_getMetadata` (~1.2 MB) return in ~9–11 s in Full mode thanks to binary wire framing and raw deflate compression.

### Can I switch modes at runtime?
Yes. Send a `blindhop_setPrivacyMode` JSON-RPC message to the proxy, or use the privacy slider in the demo UI. The proxy reconnects with a fresh transport and confirms the mode actually active.

## Operations

### How do I run the exit service?
```bash
cargo run -p blindhop-exit -- --target-rpc wss://your-substrate-node.example.com
```
The exit service stores its Nym keys in `.blindhop-exit/` (so its Nym address remains stable across restarts) and prints its address to stdout and `.exit_nym_address`. For production server setups, automated systemd scripts with resource caps and sandboxing are provided in `deploy/`.

### Do I need to run my own exit service?
No. The demo comes pre-filled with the public BlindHop exit, and the proxy can use it too (`--exit-address`). Running your own exit means you don't have to trust that operator's node choice or uptime. A registry of community-operated exits is planned.

### What are the bandwidth requirements?
The exit service handles standard JSON-RPC queries. Under load, it caps in-flight requests to 16 concurrent (8 per client), limits responses to 4 MiB, and sends replies immediately to prevent packet queueing. Resource usage is modest: in load testing (40 concurrent `state_getMetadata` requests) memory peaked at ~100 MB, and the provided systemd unit caps the exit at 320 MB and one CPU core.

### Can I use BlindHop without the Nym mixnet?
Yes, in **None mode**. The proxy forwards traffic directly without using Nym. This is useful for development and when privacy is not required.
