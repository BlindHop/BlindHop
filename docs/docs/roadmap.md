---
sidebar_position: 11
title: Roadmap
---

# Roadmap

## Phase 1: Nym Integration + Proxy/Exit MVP ✅
**Duration:** 10 weeks | **Status:** ✅ Complete

- [x] Architecture pivot: custom Sphinx → Nym SDK integration
- [x] `blindhop-common`: MixnetTransport trait, config, metrics, RPC types
- [x] `blindhop-proxy`: Nym client, WS bridge, privacy mode switching
- [x] `blindhop-exit`: Nym Service Provider, Substrate RPC forwarding
- [x] Demo UI with privacy slider (None/Fast/Full)
- [x] Unit tests (14 passing), CI/CD pipeline

**Milestone acceptance:**
- Workspace builds and tests pass
- Privacy slider switches between modes at runtime
- Exit service forwards JSON-RPC to Substrate full node

---

## Phase 2: Browser Wasm + Advanced Exit Routing
**Duration:** 8 weeks | **Status:** 🔲 Planned

- [ ] Browser-only mode: Nym Wasm client for in-browser operation
- [ ] SOCKS5 exit fallback: use Nym's built-in SOCKS5 proxy
- [ ] Multi-exit load balancing: route to multiple exit services
- [ ] Connection pooling: reuse WebSocket connections at exit
- [ ] Nym credential integration: zk-nym bandwidth tokens
- [ ] smoldot PlatformRef wrapper for seamless integration

**Milestone acceptance:**
- Browser demo works without native proxy
- SOCKS5 fallback operational when dedicated exit is unavailable
- Latency benchmarks documented

---

## Phase 3: ZK Relay Proofs + On-Chain Settlement
**Duration:** 8 weeks | **Status:** 🔲 Planned

- [ ] ZK relay proofs: Stwo Circle STARK circuits for correct relay verification
- [ ] PolkaVM smart contracts: Registry + Verifier on Asset Hub
- [ ] Proof aggregation: binary tree composition for O(log N) verification
- [ ] Slashing: fraud proofs for misbehaving operators
- [ ] Custom mixnode support: `MixnetTransport` alternative using dedicated nodes

**Milestone acceptance:**
- Root proof (~35 KB) verifies in < 100ms
- PolkaVM contracts deployed on testnet
- Custom mixnode path operational alongside Nym

---

## Phase 4: Production Hardening + Security Audit
**Duration:** 4 weeks | **Status:** 🔲 Planned

- [ ] Security review and third-party audit
- [ ] Performance optimization and stress testing
- [ ] Final documentation at wiki.blindhop.wtf
- [ ] Production deployment guide
- [ ] Bounty submission materials

**Milestone acceptance:**
- Security audit with no critical findings
- Documented deployment procedures
- End-to-end demo with real Nym mainnet traffic

---

## Future Directions (Post-Bounty)

- **Mobile SDK** — React Native / Flutter integration
- **Cross-chain mixnet** — shared exit services across parachains
- **Incentive layer** — fee market for exit service operators
- **Value privacy** — adapter for confidential transaction types
- **Post-quantum** — migration when Nym adds PQ key exchange
