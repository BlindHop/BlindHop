#!/usr/bin/env bash
# BlindHop Benchmark Script — compares latency across privacy modes
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "╔═══════════════════════════════════════════════════╗"
echo "║        BlindHop v2 — Latency Benchmark           ║"
echo "╚═══════════════════════════════════════════════════╝"
echo ""

TARGET="${TARGET_RPC:-wss://sys.turboflakes.io/asset-hub-paseo}"
ITERATIONS="${ITERATIONS:-10}"

echo "Target:     $TARGET"
echo "Iterations: $ITERATIONS"
echo ""

# Build first
cargo build --workspace --release 2>&1

echo "────────────────────────────────────────────────────"
echo "Mode: Direct (no privacy)"
echo "────────────────────────────────────────────────────"
echo "Querying $TARGET directly..."

for i in $(seq 1 "$ITERATIONS"); do
    START=$(date +%s%N)
    # Use websocat or wscat if available, otherwise skip
    if command -v websocat &> /dev/null; then
        echo '{"jsonrpc":"2.0","id":1,"method":"chain_getHeader"}' | \
            websocat -n1 "$TARGET" > /dev/null 2>&1
        END=$(date +%s%N)
        ELAPSED=$(( (END - START) / 1000000 ))
        echo "  Request $i: ${ELAPSED}ms"
    else
        echo "  websocat not found — install with: cargo install websocat"
        break
    fi
done

echo ""
echo "Note: For Nym modes (Fast/Full), start the proxy and exit service,"
echo "then run the demo UI for interactive benchmarking."
echo ""
echo "Commands:"
echo "  cargo run -p blindhop-exit -- --target-rpc $TARGET"
echo "  cargo run -p blindhop-proxy -- --privacy-mode full --exit-address <NYM_ADDR>"
