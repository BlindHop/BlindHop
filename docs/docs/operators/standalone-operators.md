---
sidebar_position: 3
title: Standalone Operators
---

# Standalone Operators

Standalone operators are independent mixnode operators who are **not** chain validators. They register via the BlindHop Registry contract and post a staking bond.

## Registration

1. **Prepare**: Generate x25519 key pair and configure your node
2. **Bond**: Transfer the minimum staking bond (e.g., 10 KSM) to the Registry contract
3. **Register**: Call `register(public_key, endpoint)` on the contract
4. **Prove**: Generate and submit your first eligibility proof
5. **Operate**: Begin relaying packets and generating cover traffic

## Staking Bond

| Parameter | Value |
|---|---|
| Minimum bond | Configurable per deployment (e.g., 10 KSM) |
| Unbonding period | 7 days after voluntary unregistration |
| Slashing conditions | Fraud proof accepted, repeated non-compliance |
| Bond return | Full return after unbonding period (if not slashed) |

## When Are Standalones Used?

Standalones are activated when the validator-only pool is **below the chain's anonymity set threshold**:

```
if validator_mixnodes.len() >= chain_min_anonymity_set {
    // Standalones are deprioritized (validators are sufficient)
} else {
    // Fill remaining slots with standalones (sorted by stake)
}
```

This ensures the mixnet always has enough operators for meaningful privacy, even on chains with few validators running BlindHop.
