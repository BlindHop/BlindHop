#!/usr/bin/env bash
# BlindHop v2 Demo — Start the proxy and serve the demo page
#
# Usage:
#   ./scripts/run_demo.sh [--mode full|fast|none] [--exit-address <NYM_ADDRESS>]
#
# Prerequisites:
#   - Rust toolchain
#   - A running blindhop-exit instance (for Fast/Full modes)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Defaults
MODE="${1:-full}"
EXIT_ADDRESS="${EXIT_ADDRESS:-}"
TARGET_RPC="${TARGET_RPC:-wss://sys.turboflakes.io/asset-hub-paseo}"
PROXY_PORT=9500
DEMO_PORT=8080

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode) MODE="$2"; shift 2 ;;
        --exit-address) EXIT_ADDRESS="$2"; shift 2 ;;
        --target) TARGET_RPC="$2"; shift 2 ;;
        *) shift ;;
    esac
done

echo "╔═══════════════════════════════════════════════════╗"
echo "║        BlindHop v2 — Nym Mixnet Demo             ║"
echo "╚═══════════════════════════════════════════════════╝"
echo ""
echo "  Privacy Mode:  $MODE"
echo "  Target RPC:    $TARGET_RPC"
echo "  Proxy:         ws://127.0.0.1:$PROXY_PORT"
echo "  Demo UI:       http://127.0.0.1:$DEMO_PORT"
echo ""

# Try to read exit address from file if not specified
if [[ -z "$EXIT_ADDRESS" ]] && [[ -f "$ROOT_DIR/.exit_nym_address" ]]; then
    EXIT_ADDRESS=$(cat "$ROOT_DIR/.exit_nym_address")
    echo "  Exit address:  $EXIT_ADDRESS (from .exit_nym_address)"
fi

# Build
echo "→ Building workspace..."
cd "$ROOT_DIR"
cargo build --workspace 2>&1

# Start proxy
echo ""
echo "→ Starting BlindHop proxy..."

PROXY_ARGS=(
    --listen "127.0.0.1:$PROXY_PORT"
    --target "$TARGET_RPC"
    --privacy-mode "$MODE"
)

if [[ -n "$EXIT_ADDRESS" ]]; then
    PROXY_ARGS+=(--exit-address "$EXIT_ADDRESS")
fi

cargo run -p blindhop-proxy -- "${PROXY_ARGS[@]}" &
PROXY_PID=$!

echo "  Proxy PID: $PROXY_PID"
sleep 2

# Serve demo page
echo ""
echo "→ Serving demo UI on http://127.0.0.1:$DEMO_PORT"
cd "$ROOT_DIR/demo"

# Use Python's built-in HTTP server
if command -v python3 &> /dev/null; then
    python3 -m http.server $DEMO_PORT &
    DEMO_PID=$!
elif command -v npx &> /dev/null; then
    npx -y serve -l $DEMO_PORT . &
    DEMO_PID=$!
else
    echo "⚠ No HTTP server found. Open demo/index.html manually."
    DEMO_PID=""
fi

echo ""
echo "╔═══════════════════════════════════════════════════╗"
echo "║  Demo running! Open http://127.0.0.1:$DEMO_PORT       ║"
echo "║  Press Ctrl+C to stop                            ║"
echo "╚═══════════════════════════════════════════════════╝"

# Cleanup on exit
cleanup() {
    echo ""
    echo "→ Shutting down..."
    [[ -n "${PROXY_PID:-}" ]] && kill "$PROXY_PID" 2>/dev/null || true
    [[ -n "${DEMO_PID:-}" ]] && kill "$DEMO_PID" 2>/dev/null || true
    echo "  Done."
}
trap cleanup EXIT

# Wait
wait
