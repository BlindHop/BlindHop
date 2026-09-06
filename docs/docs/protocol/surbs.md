---
sidebar_position: 4
title: SURBs (Anonymous Replies)
description: How Single-Use Reply Blocks enable anonymous responses
---

# SURBs — Single-Use Reply Blocks

SURBs are a key component of the Sphinx packet format that enable **anonymous replies**. They allow the exit service to respond to requests without knowing the sender's identity.

## How SURBs Work

1. When `blindhop-proxy` sends a request through the Nym mixnet, the SDK automatically attaches SURBs
2. Each SURB contains pre-encrypted routing information for the return path
3. The exit service uses `client.send_reply(sender_tag, response)` to send a reply via the SURB
4. The reply traverses the mixnet back to the sender through the pre-built path
5. Only the original sender can decrypt the reply

## SURB Properties

| Property | Description |
|----------|-------------|
| **Single-use** | Each SURB can only be used once (prevents replay) |
| **Anonymous** | The exit service never learns the sender's identity |
| **Pre-built** | Return path is constructed by the sender, not the replier |
| **Encrypted** | Each hop's routing info is encrypted for that specific node |

## In BlindHop

The `nym-sdk` handles SURB management automatically:

```rust
// Proxy side: SURBs are attached automatically by nym-sdk
client.send_plain_message(exit_address, request_bytes).await?;

// Exit side: reply via sender_tag (SURB)
if let Some(tag) = received_message.sender_tag {
    client.send_reply(tag, response_bytes).await?;
}
```

The `sender_tag` is an `AnonymousSenderTag` that the exit service uses to route the reply back through the pre-built SURB path. The exit service never learns the proxy's Nym address or IP.
