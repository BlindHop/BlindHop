---
sidebar_position: 2
title: Validator Mixnodes
---

# Validator Mixnodes

Validators who run the BlindHop plugin automatically become mixnodes with **zero additional registration**.

## MixnodePlugin Trait

The `MixnodePlugin` trait defines the integration contract between the BlindHop mixnode library and a Substrate full node:

```rust
/// Trait for integrating mixnode logic into any Substrate full node.
pub trait MixnodePlugin {
    /// Handle an incoming Sphinx packet: decrypt one layer, apply delay, forward
    fn on_sphinx_packet(&mut self, packet: SphinxPacket) -> Result<()>;

    /// Return the current session's mixnode keys (x25519)
    fn session_keys(&self) -> &MixnodeSessionKeys;

    /// Submit a decoded extrinsic to the local full node (exit node path)
    fn submit_extrinsic(&self, ext: Vec<u8>) -> Result<Hash>;

    /// Execute a storage query on the local full node (exit node path)
    fn storage_query(&self, key: StorageKey) -> Result<StorageValue>;

    /// Submit a base Stwo relay proof for aggregation
    fn submit_base_proof(&self, proof: StwoProof) -> Result<()>;

    /// Handle an aggregation request: combine two inner proofs into one
    fn on_aggregation_request(&mut self, proofs: Vec<StwoProof>) -> Result<StwoProof>;
}
```

### Basic Integration

```rust
use blindhop_full_node::MixnodePlugin;

// In your validator's main loop:
let mixnode = BlindHopMixnode::new(config);

// Handle incoming Sphinx packets
mixnode.on_sphinx_packet(packet)?;

// Session keys are derived from your validator keys
let session_keys = mixnode.session_keys();
```

## Advantages Over Standalone

| Aspect | Validator-Mixnode | Standalone |
|---|---|---|
| Registration | Automatic (from validator set) | Manual (contract call + bond) |
| Staking | Implicit (validator bond) | Explicit (separate bond) |
| Trust level | Higher (already economically bonded) | Lower (only BlindHop bond) |
| Priority | First in pool | Fills gaps |
| Eligibility proof | Merkle membership in validator set | Merkle membership in registry |

## Requirements

- Running a Substrate full node as a validator
- The `blindhop-full-node` library integrated alongside the node
- Sufficient bandwidth for Sphinx packet relay + cover traffic (~50 KB/s)
- Session keys rotated each era (handled automatically)
