---
sidebar_position: 3
title: Eligibility Proof
---

# Eligibility Proof — Mixnode Membership

The **Eligibility Proof** proves that a mixnode operator is a legitimate member of the active validator set or the standalone registry — **without revealing which specific operator they are**.

## Circuit Specification

| Aspect | Detail |
|---|---|
| **System** | Stwo Circle STARK (M31 field) |
| **Constraints** | ~30,000 AIR constraints |
| **Proving time** | ~30 ms (native) |
| **Proof size** | ~6 KB |

## Public Inputs

| Input | Description |
|---|---|
| `merkle_root` | Poseidon2 Merkle root of the combined validator set + standalone registry |
| `epoch_number` | Current epoch/session index |

## Private Inputs

| Input | Description |
|---|---|
| `operator_secret_key` | The operator's private key |
| `operator_public_key` | The operator's public key (leaf in the Merkle tree) |
| `merkle_proof` | Sibling hashes along the path from leaf to root |
| `leaf_index` | Position in the Merkle tree |

## What the Circuit Proves

1. **Key ownership**: The prover knows the secret key corresponding to a public key in the tree
2. **Merkle membership**: The public key is a valid leaf in the Merkle tree with the given root
3. **Epoch binding**: The proof is bound to the current epoch (prevents replay)

$$
\text{Poseidon2}(\text{leaf}) \in \text{MerkleTree}(\text{root})
$$

## Verification Modes

### Peer-to-Peer (Real-Time)

Every session, mixnodes present their eligibility proof to peers:

- Verified by existing mixnodes in < 50ms
- Accepted into the cascade immediately upon verification
- No on-chain transaction required

### On-Chain Checkpoint (Periodic)

Every N sessions, mixnodes submit their proof on-chain:

- Verified by the Stwo verifier contract on PolkaVM
- Creates an immutable record for dispute resolution
- Required for standalone operators (validators can reference chain state)

## Merkle Tree Construction

The combined Merkle tree is built from two sources:

```
                    [Root]
                   /      \
            [Validators]  [Standalones]
            /    \          /    \
        [V1]  [V2]     [S1]  [S2]
```

- **Left subtree**: Active validator set (read from chain state)
- **Right subtree**: Registered standalone operators (read from Registry contract)
- **Hash function**: Poseidon2 over M31
- **Rebuilt**: Every session when the validator set changes
