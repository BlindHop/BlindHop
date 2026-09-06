---
sidebar_position: 1
title: Threat Model
---

# Threat Model

## Adversary Capabilities

BlindHop considers the following adversary types:

| Adversary | Capability | Goal |
|---|---|---|
| **Global Passive Adversary (GPA)** | Observes all network links | Correlate sender with transaction |
| **Malicious Mixnode** | Controls up to f < N/3 mixnodes | Deanonymize users by controlling path |
| **Malicious Full Node** | Operates exit points | Link IP to transaction content |
| **ISP/Network Observer** | Monitors user's connection | Determine if/when user transacts |
| **Quantum Adversary** | Future quantum computer | Break cryptographic assumptions |

## Security Goals

| Goal | Mechanism |
|---|---|
| **Sender anonymity** | Sphinx packet re-encryption + re-blinding at each hop |
| **Receiver anonymity** | SURBs hide the return path from the exit node |
| **Relationship anonymity** | Multiple hops prevent any single node from linking sender ↔ receiver |
| **Unobservability** | Cover traffic makes active users indistinguishable from idle users |
| **Integrity** | ZK relay proofs guarantee correct packet processing |
| **Forward secrecy** | Session keys rotate each era; old keys are destroyed |
| **Post-quantum security** | Stwo Circle STARKs use hash-based commitments |

## What BlindHop Does NOT Protect Against

| Limitation | Explanation |
|---|---|
| **Application-layer correlation** | If a dApp leaks identity through transaction content (e.g., known account), BlindHop cannot prevent it |
| **Side-channel attacks** | Timing attacks on the client device itself (CPU cache, etc.) |
| **Endpoint compromise** | If the user's device is compromised, all privacy is lost |
| **> N/3 colluding mixnodes** | If an adversary controls a supermajority of mixnodes on the user's path |
| **Long-term traffic analysis** | Persistent statistical analysis over weeks/months may degrade anonymity |

## Trust Assumptions

| Component | Trust Level | Justification |
|---|---|---|
| Mixnode identity | **Trustless** | ZK eligibility proof (Stwo) |
| Packet relay | **Trustless** | ZK relay proof (Stwo) |
| Cover traffic | **Trustless** | ZK compliance proof + slashing |
| On-chain verification | **Trustless** | Same verifier code on-chain and off-chain |
| Proof availability | **Trustless** | Self-authenticating via Blake3 hash on-chain |
| Trusted setup | **None needed** | Stwo Circle STARKs are fully transparent |
