---
sidebar_position: 11
title: Roadmap
---

# Roadmap

## Phase 1: Mixnet Core + Transaction Origin Hiding
**Duration:** 10 weeks | **Status:** 🔲 Planned

- [x] Architecture design and documentation
- [ ] `blindhop-lib`: Sphinx packet encode/decode, x25519, AES-CTR, SURBs
- [ ] `blindhop-lib`: Loopix cover traffic generator, exponential delays
- [ ] `blindhop-lib`: Mixnode identity, routing table, session management
- [ ] `blindhop-light-base`: MixnetPlatform PlatformRef wrapper
- [ ] `blindhop-wasm-node`: Wasm bindings, JS API, TypeScript definitions
- [ ] `blindhop-full-node`: Sphinx relay engine, delay queue, exit node
- [ ] Integration test: browser → 3-hop mixnet → on-chain transaction

**Milestone acceptance:**
- Wireshark capture shows uniform 2 KB packets at Poisson intervals
- Transaction appears on-chain with no IP correlation to sender
- Cover traffic indistinguishable from real traffic

---

## Phase 2: ZK Relay Proofs + Metadata Privacy
**Duration:** 8 weeks | **Status:** 🔲 Planned

- [ ] `blindhop-zk`: Stwo Circle STARK relay proof circuit
- [ ] `blindhop-zk`: Binary proof tree aggregation circuit
- [ ] `blindhop-zk`: ZK eligibility proof circuit
- [ ] Metadata privacy: route ALL traffic through mixnet
- [ ] Kademlia DHT integration for proof storage
- [ ] Async proof retrieval in light client
- [ ] Performance benchmarks

**Milestone acceptance:**
- Root proof (~35 KB) verifies off-chain in < 100ms
- Storage queries return correct results through mixnet
- Eligibility proofs verified P2P in < 50ms

---

## Phase 3: Full ZK Suite + On-Chain Settlement
**Duration:** 8 weeks | **Status:** 🔲 Planned

- [ ] `blindhop-zk`: TX validity proof circuit (Wasm-feasible)
- [ ] `blindhop-zk`: Cover compliance proof circuit
- [ ] `blindhop-contracts`: Registry contract (PolkaVM)
- [ ] `blindhop-contracts`: Stwo verifier contract (PolkaVM)
- [ ] `blindhop-contracts`: rEVM adapter for MetaMask
- [ ] Hybrid eligibility: P2P + on-chain verification
- [ ] `@blindhop/client` npm package published

**Milestone acceptance:**
- TX validity proof in browser Wasm < 3 seconds
- PolkaVM verification < 10ms
- Slashing operational for non-compliant mixnodes

---

## Phase 4: Advanced Privacy Features + Production Hardening
**Duration:** 4 weeks | **Status:** 🔲 Planned

- [ ] Value-privacy adapter layer (pluggable confidential transaction integration)
- [ ] Unified privacy UI (browser demo)
- [ ] Security review and audit
- [ ] Production hardening (edge cases, stress tests)
- [ ] Final documentation at wiki.blindhop.wtf
- [ ] Bounty submission materials

**Milestone acceptance:**
- Value-privacy flows routed through mixnet successfully
- All 4 ZK proof types operational end-to-end
- Security review with no critical findings

---

## Future Directions (Post-Bounty)

- **Mobile SDK** — React Native / Flutter integration
- **Cross-chain mixnet** — shared mixnet across parachains (XCM-aware)
- **Incentive layer** — fee market for mixnode operators
- **Hardware acceleration** — GPU-accelerated Stwo proving
- **Formal verification** — machine-checked proofs of circuit correctness
