---
sidebar_position: 11
title: Roadmap
---

# Roadmap

## Phase 1: Nym Integration + Proxy/Exit MVP ✅
**Duration:** 10 weeks | **Status:** ✅ Complete

- [x] Architecture pivot: custom Sphinx → Nym SDK integration
- [x] `blindhop-common`: MixnetTransport trait (`request()`), config, metrics, binary framing with deflate compression
- [x] `blindhop-proxy`: Ephemeral Nym client, WS bridge with browser origin check, privacy mode switching
- [x] `blindhop-exit`: Hardened Nym Service Provider with persistent keys, allowlist policy, limiter, and connection pooling
- [x] In-browser mixnet client: `@nymproject/sdk-full-fat` Wasm client in demo UI
- [x] Production deployment: systemd service unit, key backups, and install scripts in `deploy/`
- [x] Automated test suite (63 offline tests across the workspace), CI/CD pipeline, dependency audit

**Milestone acceptance:**
- Workspace builds and tests pass
- Privacy slider switches between modes at runtime
- Exit service forwards JSON-RPC to Substrate full node with allowlist and concurrency protections
- Browser demo works without native proxy

---

## Phase 2: Advanced Routing + Smoldot SDK
**Duration:** 8 weeks | **Status:** 🔲 In Progress

- [x] In-browser Wasm mixnet client (delivered in MVP demo)
- [x] Connection pooling: reuse WebSocket connections at exit (`UpstreamPool`)
- [ ] Live integration tests: implement the `nym_integration` and `e2e_chain` stubs and run them nightly against a test exit
- [ ] Standalone `@blindhop/browser` SDK package
- [ ] SOCKS5 exit fallback: use Nym's built-in SOCKS5 proxy
- [ ] Multi-exit load balancing & automatic failover
- [ ] Nym credential integration: zk-nym bandwidth tokens
- [ ] smoldot PlatformRef wrapper for zero-config integration

**Milestone acceptance:**
- SOCKS5 fallback operational when dedicated exit is unavailable
- Standalone browser SDK published to npm
- Multi-exit routing operational

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
