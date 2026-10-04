#!/usr/bin/env bash
# BlindHop — Start Local Proxy
#
# Usage:
#   ./scripts/run_proxy.sh --exit-address <NYM_ADDRESS>
#   ./scripts/run_proxy.sh --mode fast --exit-address <NYM_ADDRESS>
#   ./scripts/run_proxy.sh --mode none --target wss://rpc.polkadot.io
#   ./scripts/run_proxy.sh --release --exit-address <NYM_ADDRESS>
#
# The proxy accepts WebSocket connections from smoldot/browser clients
# and routes JSON-RPC traffic through the Nym mixnet (or directly).
#
# Connect your app to ws://127.0.0.1:9500 after starting the proxy.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# Defaults
LISTEN="127.0.0.1:9500"
TARGET_RPC="wss://sys.turboflakes.io/asset-hub-paseo"
MODE="full"
EXIT_ADDRESS=""
BUILD_MODE="debug"
LOG_LEVEL="${RUST_LOG:-info}"
ALLOWED_ORIGINS=()

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --listen)         LISTEN="$2"; shift 2 ;;
        --target)         TARGET_RPC="$2"; shift 2 ;;
        --mode)           MODE="$2"; shift 2 ;;
        --exit-address)   EXIT_ADDRESS="$2"; shift 2 ;;
        --allowed-origin) ALLOWED_ORIGINS+=("$2"); shift 2 ;;
        --release)        BUILD_MODE="release"; shift ;;
        --log)            LOG_LEVEL="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --listen <ADDR>          Proxy listen address (default: $LISTEN)"
            echo "  --target <URL>           Substrate RPC endpoint (default: $TARGET_RPC)"
            echo "  --mode <MODE>            Privacy mode: none|fast|full (default: full)"
            echo "  --exit-address <ADDR>    Nym address of exit service (required for fast/full)"
            echo "  --allowed-origin <URL>   Allowed browser origin, e.g. https://demo.blindhop.wtf (repeatable)"
            echo "  --release                Build in release mode"
            echo "  --log <LEVEL>            Log level: trace|debug|info|warn|error (default: info)"
            echo "  -h, --help               Show this help"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Try to read exit address from file if not specified
if [[ -z "$EXIT_ADDRESS" ]] && [[ -f "$ROOT_DIR/.exit_nym_address" ]]; then
    EXIT_ADDRESS=$(cat "$ROOT_DIR/.exit_nym_address")
    echo "  (Read exit address from .exit_nym_address)"
fi

# Validate exit address for non-direct modes
if [[ "$MODE" != "none" ]] && [[ -z "$EXIT_ADDRESS" ]]; then
    echo "Error: --exit-address is required for '$MODE' mode."
    echo ""
    echo "Either:"
    echo "  1. Pass --exit-address <NYM_ADDRESS>"
    echo "  2. Run the exit service first (./scripts/run_exit.sh) to create .exit_nym_address"
    echo "  3. Use --mode none for direct (non-private) connections"
    exit 1
fi

echo "╔═══════════════════════════════════════════════════╗"
echo "║         BlindHop Proxy (Local)                    ║"
echo "╚═══════════════════════════════════════════════════╝"
echo ""
echo "  Listen:        ws://$LISTEN"
echo "  Target RPC:    $TARGET_RPC"
echo "  Privacy mode:  $MODE"
if [[ -n "$EXIT_ADDRESS" ]]; then
    echo "  Exit address:  ${EXIT_ADDRESS:0:32}..."
fi
echo "  Build mode:    $BUILD_MODE"
echo "  Log level:     $LOG_LEVEL"
echo ""

# Build
cd "$ROOT_DIR"
if [[ "$BUILD_MODE" == "release" ]]; then
    echo "→ Building proxy (release)..."
    cargo build -p blindhop-proxy --release 2>&1
    BINARY="./target/release/blindhop-proxy"
else
    echo "→ Building proxy (debug)..."
    cargo build -p blindhop-proxy 2>&1
    BINARY="./target/debug/blindhop-proxy"
fi

# Assemble args
PROXY_ARGS=(
    --listen "$LISTEN"
    --target "$TARGET_RPC"
    --privacy-mode "$MODE"
)

if [[ -n "$EXIT_ADDRESS" ]]; then
    PROXY_ARGS+=(--exit-address "$EXIT_ADDRESS")
fi

# ${arr[@]+...} keeps an empty array safe under `set -u` on bash 3.2 (macOS).
for origin in ${ALLOWED_ORIGINS[@]+"${ALLOWED_ORIGINS[@]}"}; do
    PROXY_ARGS+=(--allowed-origin "$origin")
done

echo ""
echo "→ Starting proxy at ws://$LISTEN"
echo "  Connect your app to this address."
echo ""

RUST_LOG="$LOG_LEVEL" exec "$BINARY" "${PROXY_ARGS[@]}"
