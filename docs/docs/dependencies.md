---
sidebar_position: 14
title: Dependencies
---

# Key Dependencies

## Rust Crates

| Crate | Purpose | Version | Used By |
|---|---|---|---|
| `smoldot-light` | Light client core (wrapped, not forked) | latest | `blindhop-light-base` |
| `stwo-prover` | Circle STARK proving system (Mersenne31) | latest | `blindhop-zk` |
| `curve25519-dalek` | x25519 key exchange for Sphinx | 4.x | `blindhop-lib` |
| `aes` + `ctr` | AES-CTR stream cipher for Sphinx payload encryption | 0.8.x | `blindhop-lib` |
| `blake3` | Network hashing (outside ZK circuits) | 1.x | `blindhop-lib`, all crates |
| `poseidon2` (M31) | In-circuit hashing (inside ZK circuits) | latest | `blindhop-zk`, `blindhop-lib` |
| `rand` + `rand_distr` | Poisson/exponential distribution sampling | 0.8.x | `blindhop-lib` |
| `wasm-bindgen` | Rust → Wasm bindings | 0.2.x | `blindhop-wasm-node` |
| `tokio` | Async runtime (native, non-Wasm) | 1.x | `blindhop-full-node` |
| `libp2p-kad` | Kademlia DHT for proof storage | latest | `blindhop-full-node` |

## JavaScript / npm

| Package | Purpose | Version |
|---|---|---|
| `@blindhop/client` | Published BlindHop client (our package) | 0.1.x |
| `typescript` | Type definitions for the JS API | 5.x |

## Build Tooling

| Tool | Purpose |
|---|---|
| `wasm-pack` | Build Wasm binaries, run browser tests |
| `cargo` | Rust workspace build system |
| `riscv32emac-unknown-none-polkavm` target | Compile contracts to PolkaVM bytecode |

## Why These Choices?

### `stwo-prover` over alternatives
- **vs. Plonky3**: Stwo has native M31 support and recursive verification built-in
- **vs. Halo2**: No trusted setup needed (Stwo is fully transparent)
- **vs. Circom/Groth16**: Post-quantum secure, no ceremony

### `curve25519-dalek` over alternatives
- **vs. ring**: `dalek` is pure Rust, `no_std` compatible, audited
- **vs. x25519-dalek**: We use `curve25519-dalek` directly for group element re-blinding (Sphinx requires this)

### `blake3` over alternatives
- **vs. SHA-256**: ~6x faster, tree-hashable, keyed mode available
- **vs. SHA-3**: ~3x faster on x86_64, simpler API
- **vs. Poseidon2**: Not ZK-friendly; we use Poseidon2 *inside* circuits and Blake3 *outside*
