<p align="center">
  <strong>BlindHop</strong><br/>
  <em>Mixnet Privacy Layer for Smoldot Light Clients</em>
</p>

<p align="center">
  <a href="https://wiki.blindhop.wtf">Documentation</a> ·
  <a href="https://blindhop.wtf">Website</a> ·
  <a href="https://www.npmjs.com/package/@blindhop/client">npm</a> ·
  <a href="PROPOSAL.md">Bounty Proposal</a>
</p>

---

## What is BlindHop?

BlindHop is a **network-layer privacy system** for Substrate-based light clients. It wraps [smoldot](https://github.com/smol-dot/smoldot) with a Sphinx/Loopix mixnet, hiding who is submitting transactions, what they're querying, and when they're active — without any trusted setup, trusted hardware, or chain fork.

**Problem:** Substrate light clients connect directly to full nodes over libp2p, exposing the user's IP address. An ISP, network observer, or malicious full node can trivially correlate transactions to real-world identities.

**Solution:** BlindHop wraps smoldot's `PlatformRef` trait, routing all traffic through a multi-hop mixnet with:
- **Sphinx packets** — uniform 2 KB, preventing traffic analysis
- **Poisson cover traffic** — real and dummy packets are indistinguishable
- **Stwo Circle STARKs** — every mixnode proves correct relay with zero-knowledge proofs
- **PolkaVM verification** — root proofs verified on-chain, no new pallets required

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                    Tier 1: Edge Layer (Browser)                    │
│  @blindhop/client → MixnetPlatform<P> wraps smoldot PlatformRef   │
│  Sphinx packets · Poisson cover traffic · SURBs · TX validity ZKP │
└────────────────────────┬───────────────────────────────────────────┘
                         │ 2 KB uniform packets
┌────────────────────────▼───────────────────────────────────────────┐
│                  Tier 2: Core Network (Mixnodes)                   │
│  Validators (priority) + Standalone operators (fill gaps)          │
│  Per-hop: decrypt → delay → forward → generate Stwo relay proof   │
│  Binary Proof Tree: O(log N) aggregation → ~35 KB root proof      │
└────────────────────────┬───────────────────────────────────────────┘
                         │ Blake3 root hash (128 bytes)
┌────────────────────────▼───────────────────────────────────────────┐
│                  Tier 3: Settlement (PolkaVM)                       │
│  Registry Contract: staking, registration, slashing               │
│  Verifier Contract: Stwo M31 sumcheck (< 10ms, JIT)               │
│  Zero new pallets — Rust no_std → RISC-V → PolkaVM bytecode       │
└────────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
blindhop/
├── lib/              # blindhop-lib — Sphinx, Loopix, SURB, Poseidon2/Blake3
├── light-base/       # blindhop-light-base — MixnetPlatform<P>, Builder, Handle
├── wasm-node/        # blindhop-wasm-node — JS/Wasm bindings, @blindhop/client
├── full-node/        # blindhop-full-node — Mixnode relay, exit, aggregation
├── zk/               # blindhop-zk — Stwo circuits (4 types) + recursion
├── contracts/        # blindhop-contracts — Registry + Verifier (PolkaVM)
└── tests/            # Integration tests
```

## Quick Start

### For dApp Developers

```bash
npm install @blindhop/client
```

```typescript
import { BlindHop } from '@blindhop/client';

const chainSpec = await fetch('/kusama.json').then(r => r.text());

const client = await BlindHop.start({
  chainSpec,
  hopCount: 3,              // 1-5 hops (privacy/latency tradeoff)
  privacyMode: 'required',  // 'required' | 'best_effort'
  coverTrafficRate: 1.0,    // packets/sec
});

// Use like a normal smoldot client — privacy is transparent
const chain = await client.addChain({ chainSpec });
```

### For Mixnode Operators

#### Validator Mixnodes (automatic)
Validators who run the BlindHop plugin become mixnodes with zero additional registration:

```rust
use blindhop_full_node::MixnodePlugin;

let mixnode = BlindHopMixnode::new(config);
mixnode.on_sphinx_packet(packet)?;
```

#### Standalone Operators
Register via PolkaVM contract + post staking bond:

```rust
use blindhop_contracts::Registry;

let registry = Registry::connect(rpc_url).await?;
registry.register(bond_amount, mixnode_public_key).await?;
```

## ZK Proofs (Stwo Circle STARKs)

| Proof | Purpose | Generator | Size |
|---|---|---|---|
| Relay Proof | Correct Sphinx decryption + forwarding | Mixnode | ~8 KB base |
| Eligibility Proof | Validator/registry membership (privacy-preserving) | Mixnode | ~5 KB |
| TX Validity Proof | Extrinsic well-formedness | Light client (Wasm) | ~3 KB |
| Cover Compliance | Required cover traffic volume generated | Mixnode | ~4 KB |

All proofs use **Poseidon2** (M31) inside circuits and **Blake3** outside. Base proofs are aggregated via a **Recursive Binary Proof Tree** into a single **~35 KB root proof**, with only a **128-byte Blake3 hash** committed on-chain.

## Performance

| Metric | Value |
|---|---|
| Packet size | 2,048 bytes (fixed) |
| Round-trip (3-hop) | ~5.1 seconds |
| Root proof size | ~35 KB |
| On-chain commitment | 128 bytes |
| PolkaVM verification | < 10ms |
| Wasm TX proof generation | < 3 seconds |
| Cover traffic overhead | ~50 KB/s per node |

## Documentation

Full documentation is hosted at **[wiki.blindhop.wtf](https://wiki.blindhop.wtf)**:

| Section | Topics |
|---|---|
| [Getting Started](https://wiki.blindhop.wtf/getting-started) | Installation, browser/Node.js usage, demo page |
| [Architecture](https://wiki.blindhop.wtf/architecture/overview) | 3-tier system, crate structure, data flow |
| [Mixnet Protocol](https://wiki.blindhop.wtf/protocol/sphinx) | Sphinx, Loopix, SURBs, cover traffic, sessions |
| [ZK Proofs](https://wiki.blindhop.wtf/zk/stwo-overview) | All 4 circuits, binary proof tree, dual hash strategy |
| [Smart Contracts](https://wiki.blindhop.wtf/contracts/registry) | Registry, verifier, rEVM adapter |
| [Operators](https://wiki.blindhop.wtf/operators/overview) | Validator mixnodes, standalone operators, threshold, slashing |
| [API Reference](https://wiki.blindhop.wtf/api/builder) | Builder, Handle, Config, JavaScript API |
| [Security](https://wiki.blindhop.wtf/security/threat-model) | Threat model, traffic analysis, trustless guarantees |
| [Performance](https://wiki.blindhop.wtf/performance/benchmarks) | Benchmarks, latency analysis, optimization |
| [Testing](https://wiki.blindhop.wtf/testing) | Automated tests, local testnet, verification checklist |

## Building & Testing

### Prerequisites

- **Rust** (stable, with `wasm32-unknown-unknown` and `riscv32emac-unknown-none-polkavm` targets)
- **Node.js** ≥ 18
- **wasm-pack**

### Build

```bash
# Build all Rust crates
cargo build --workspace

# Build Wasm bindings
cd wasm-node && wasm-pack build --target web

# Build PolkaVM contracts
cargo build -p blindhop-contracts --target riscv32emac-unknown-none-polkavm
```

### Test

```bash
# Unit tests
cargo test --workspace

# Integration tests (requires local testnet)
cargo test -p blindhop-integration --features local-testnet

# Browser tests
wasm-pack test --headless --chrome blindhop-wasm-node
```

### Documentation Site (Local)

```bash
cd docs
npm install
npm run start     # Dev server at http://localhost:3000
npm run build     # Production build to docs/build/
```

## Deploying Documentation

The documentation site is automatically deployed to GitHub Pages via the [deploy-docs workflow](.github/workflows/deploy-docs.yml) on every push to `main` that modifies `docs/`.

### Manual Setup (one-time)

1. **GitHub repo** → Settings → Pages → Source: **GitHub Actions**
2. **Custom domain**: Add `wiki.blindhop.wtf` in the Pages settings
3. **DNS**: Create a CNAME record:
   ```
   wiki.blindhop.wtf → <your-github-username>.github.io
   ```
4. Push to `main` — the workflow builds and deploys automatically

## Development Timeline

| Phase | Duration | Focus |
|---|---|---|
| **Phase 1** | Weeks 1–10 | Mixnet core + transaction origin hiding |
| **Phase 2** | Weeks 11–18 | ZK relay proofs + metadata privacy |
| **Phase 3** | Weeks 19–26 | Full ZK suite + on-chain settlement |
| **Phase 4** | Weeks 27–30 | Advanced privacy features + production hardening |

See [timeline](https://wiki.blindhop.wtf/timeline) for the week-by-week breakdown.

## Key Dependencies

| Crate | Purpose |
|---|---|
| `smoldot-light` | Light client core (wrapped, not forked) |
| `stwo-prover` | Circle STARK proving (Mersenne31) |
| `curve25519-dalek` | x25519 key exchange for Sphinx |
| `blake3` | Network hashing |
| `poseidon2` (M31) | In-circuit hashing |

See [dependencies](https://wiki.blindhop.wtf/dependencies) for the full list with version pins and rationale.

## License

Apache 2.0 + MIT dual license — same as smoldot and the Substrate ecosystem.
