---
sidebar_position: 5
title: Cover Compliance Proof
---

# Cover Compliance Proof

The **Cover Compliance Proof** proves that a mixnode generated the required volume of cover traffic during an epoch — without revealing actual traffic patterns or timing.

## Circuit Specification

| Aspect | Detail |
|---|---|
| **System** | Stwo Circle STARK (M31 field) |
| **Constraints** | ~10,000 AIR constraints |
| **Proving time** | ~20 ms (native) |
| **Proof size** | ~3 KB |

## Public Inputs

| Input | Description |
|---|---|
| `epoch_number` | The epoch being proven |
| `required_count` | Minimum cover packets required |
| `traffic_log_commitment` | Poseidon2 hash of the traffic log |

## Private Inputs

| Input | Description |
|---|---|
| `cover_packet_timestamps` | Timestamps of each generated cover packet |
| `cover_packet_routes` | Routes used for each cover packet |
| `total_count` | Actual number of cover packets generated |

## What It Proves

1. **Volume compliance**: `total_count ≥ required_count`
2. **Temporal distribution**: Packets are spread across the epoch (not burst-generated)
3. **Log integrity**: The traffic log commitment matches the actual traffic data

## Enforcement

Cover compliance proofs are submitted **on-chain** to the Registry contract. Failure to submit a valid proof within the deadline triggers:

1. Warning period (1 epoch grace)
2. Stake reduction (10% of bond)
3. Eviction from the mixnode pool (if repeated)
