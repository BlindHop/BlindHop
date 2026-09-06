---
sidebar_position: 1
title: Registry Contract
---

# BlindHop Registry Contract

The **Registry Contract** is the unified on-chain smart contract that manages all BlindHop state on Asset Hub. It handles standalone operator registration, staking, eligibility checkpoints, cover compliance, and slashing.

## Deployment

- **Language**: Rust `#![no_std]`
- **Target**: RISC-V → `.polkavm` bytecode
- **Runtime**: `pallet-revive` on Asset Hub
- **Deployment**: Permissionless via extrinsic

## Contract Interface

```rust
/// BlindHop Registry — PolkaVM Smart Contract
#[no_std]
pub trait BlindHopRegistry {
    /// Register as a standalone mixnode operator
    /// Requires staking bond (transferred with the call)
    fn register(public_key: [u8; 32], endpoint: SocketAddr) -> Result<()>;

    /// Unregister and begin unbonding period
    fn unregister() -> Result<()>;

    /// Submit eligibility checkpoint proof (every N sessions)
    fn submit_eligibility_checkpoint(proof_hash: [u8; 32]) -> Result<()>;

    /// Submit cover compliance proof for an epoch
    fn submit_cover_compliance(epoch: u32, proof_hash: [u8; 32]) -> Result<()>;

    /// Submit root proof checkpoint (Blake3 hash of ~35 KB proof)
    fn submit_root_proof_checkpoint(root_hash: [u8; 32]) -> Result<()>;

    /// Submit fraud proof to slash a misbehaving operator
    fn submit_fraud_proof(target: AccountId, proof: Vec<u8>) -> Result<()>;

    /// Read the list of registered standalone operators
    fn get_operators() -> Vec<OperatorInfo>;

    /// Get the minimum staking bond
    fn get_min_bond() -> Balance;

    /// Get the current epoch's cover traffic requirements
    fn get_cover_requirements(epoch: u32) -> CoverRequirements;
}
```

## State Layout

```rust
/// On-chain state stored by the Registry contract
struct RegistryState {
    /// Registered standalone operators
    operators: BTreeMap<AccountId, OperatorInfo>,

    /// Eligibility checkpoints (operator → last checkpoint session)
    eligibility_checkpoints: BTreeMap<AccountId, u32>,

    /// Cover compliance records (operator → last compliant epoch)
    cover_compliance: BTreeMap<AccountId, u32>,

    /// Root proof checkpoints (hash → verification status)
    root_proofs: BTreeMap<[u8; 32], ProofStatus>,

    /// Configuration
    min_bond: Balance,
    checkpoint_interval: u32,   // sessions between required checkpoints
    cover_deadline: u32,        // blocks after epoch end to submit proof
}

struct OperatorInfo {
    account: AccountId,
    public_key: [u8; 32],       // x25519 session public key
    endpoint: SocketAddr,
    bond: Balance,
    registered_at: BlockNumber,
    status: OperatorStatus,     // Active | Unbonding | Slashed
}
```

## Staking & Slashing

| Action | Consequence |
|---|---|
| **Register** | Must transfer `min_bond` (configurable, e.g., 10 KSM) |
| **Miss eligibility checkpoint** | Warning → 10% bond reduction → eviction |
| **Miss cover compliance** | Warning → 10% bond reduction → eviction |
| **Fraud proof accepted** | 100% bond slashed, operator permanently banned |
| **Voluntary unregister** | Unbonding period (e.g., 7 days), then bond returned |
