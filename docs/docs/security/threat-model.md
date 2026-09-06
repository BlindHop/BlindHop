---
sidebar_position: 1
title: Threat Model
---

# Threat Model

## Adversary Capabilities

BlindHop inherits the Nym mixnet's threat model. We consider:

| Adversary | Can Observe | Cannot Determine |
|-----------|------------|-----------------|
| **ISP / Network observer** | That you're using Nym | What you're sending, to whom |
| **Malicious full node** | JSON-RPC requests from exit IP | Who sent the request |
| **Malicious exit service** | JSON-RPC payload content | Sender's IP or identity |
| **Malicious mix node** | Encrypted packets passing through | Content, sender, or recipient |
| **Colluding nodes (< all)** | Partial traffic flow | Full sender-recipient correlation |

## Privacy Mode Guarantees

| Threat | None | Fast | Full |
|--------|------|------|------|
| Full node learns your IP | ✗ Protected | ✓ Protected | ✓ Protected |
| ISP sees your queries | ✗ Protected | ⚠️ Partially | ✓ Protected |
| Traffic timing analysis | ✗ Protected | ✗ Protected | ✓ Protected |
| Activity detection | ✗ Protected | ✗ Protected | ✓ Protected |

## Key Assumptions

1. **Nym network is honest-majority** — at least one mix node per layer is honest
2. **Sphinx packets are cryptographically sound** — x25519 + AES are unbroken
3. **Cover traffic is sufficient** — Loopix parameters provide adequate noise
4. **Exit service is semi-trusted** — it sees content but not sender identity

## Limitations

- **Full mode** provides strong anonymity but with 1-3s latency overhead
- **Fast mode** hides IP but does not protect against traffic analysis
- **None mode** provides zero privacy (direct connection)
- The exit service sees plaintext JSON-RPC (but not who sent it)
