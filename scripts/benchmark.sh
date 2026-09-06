#!/usr/bin/env bash
set -euo pipefail

echo "=== BlindHop Benchmark Suite ==="
echo ""

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo "1. Running unit tests..."
cargo test --workspace 2>&1 | tail -5
echo ""

echo "2. Running Criterion micro-benchmarks..."
echo "   (Results saved to target/criterion/)"
cargo bench -p blindhop-lib 2>&1 | grep -E "time:|Benchmarking"
echo ""

echo "3. Summary"
echo "=========================================="
echo ""
echo "Benchmark results are in: target/criterion/"
echo ""
echo "Key metrics to look for:"
echo "  - sphinx_create/1_hop:    Sphinx packet creation (1 hop)"
echo "  - sphinx_create/3_hop:    Sphinx packet creation (3 hops)"
echo "  - sphinx_process_relay:   Per-hop relay processing"
echo "  - blake3_hkdf_derive:     Key derivation"
echo "  - blake3_mac_512b:        MAC computation"
echo "  - aes_ctr_encrypt_3layers_1kb: Payload encryption"
echo ""
echo "To view detailed HTML reports:"
echo "  open target/criterion/report/index.html"
