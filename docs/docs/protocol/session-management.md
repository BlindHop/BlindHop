---
sidebar_position: 5
title: Session Management
description: How Nym manages keys and sessions
---

# Session Management

## Nym Client Sessions

The Nym SDK manages cryptographic sessions automatically. When `blindhop-proxy` or `blindhop-exit` connects to the Nym mixnet, the SDK:

1. **Generates a client identity** — x25519 keypair for Sphinx operations
2. **Selects a gateway** — connects to a Nym gateway node
3. **Registers with the network** — announces availability for message receipt
4. **Manages key rotation** — handles periodic key updates

## BlindHop Session Lifecycle

```mermaid
sequenceDiagram
    participant Proxy as blindhop-proxy
    participant Nym as Nym Network
    participant Exit as blindhop-exit

    Proxy->>Nym: MixnetClientBuilder::new_ephemeral()
    Nym-->>Proxy: Ephemeral Client ID + gateway assignment

    Exit->>Nym: MixnetClientBuilder::new_with_default_storage(.blindhop-exit)
    Nym-->>Exit: Persistent SP address (stable across restarts)

    Note over Proxy,Exit: Exit keys are kept on disk; proxy uses ephemeral sessions
    Note over Proxy,Exit: Watchdog tasks detect unexpected SDK termination and trigger supervisor restart
```

## Privacy Mode Switching

When the user switches privacy modes (via slider or `blindhop_setPrivacyMode`), the proxy handles the transition:

| Transition | Action |
|-----------|--------|
| None → Fast/Full | Connects a new `NymTransport` configured for the target mode |
| Fast ↔ Full | Disconnects old transport and connects a fresh ephemeral client with updated hop and cover traffic parameters |
| Fast/Full → None | Drops mixnet transport and switches to direct Substrate WebSocket forwarding |

## Persistence & Key Management

- **Exit Service**: The exit stores its Nym keys on disk in `--data-dir` (default: `.blindhop-exit/`, or `/var/lib/blindhop-exit/keys` under systemd). Preserving this directory is essential: it keeps the exit's Nym address stable across restarts and upgrades so clients don't lose connection.
- **Proxy**: Uses ephemeral in-memory keys (`new_ephemeral()`). Smoldot and browser users do not need a persistent address; anonymous reply SURBs allow the exit to reply without storing user identities.
- **Browser Client**: In-browser Wasm clients generate ephemeral session keys in a Web Worker, which are discarded when the tab closes.
