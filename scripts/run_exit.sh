#!/usr/bin/env bash
# BlindHop — Start Exit Service (Nym Service Provider)
#
# Usage:
#   ./scripts/run_exit.sh
#   ./scripts/run_exit.sh --target wss://rpc.polkadot.io
#   ./scripts/run_exit.sh --release
#
# The exit service connects to the Nym mixnet as a Service Provider,
# receives JSON-RPC requests through the mixnet, forwards them to a
# Substrate full node, and returns responses via SURB reply channels.
#
# On startup it prints its Nym address and writes it to .exit_nym_address.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# Defaults
TARGET_RPC="wss://sys.turboflakes.io/asset-hub-paseo"
BUILD_MODE="debug"
LOG_LEVEL="${RUST_LOG:-info}"

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)     TARGET_RPC="$2"; shift 2 ;;
        --release)    BUILD_MODE="release"; shift ;;
        --log)        LOG_LEVEL="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --target <URL>   Substrate RPC endpoint (default: $TARGET_RPC)"
            echo "  --release        Build in release mode"
            echo "  --log <LEVEL>    Log level: trace|debug|info|warn|error (default: info)"
            echo "  -h, --help       Show this help"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo "╔═══════════════════════════════════════════════════╗"
echo "║      BlindHop Exit Service (Nym SP)               ║"
echo "╚═══════════════════════════════════════════════════╝"
echo ""
echo "  Target RPC:   $TARGET_RPC"
echo "  Build mode:   $BUILD_MODE"
echo "  Log level:    $LOG_LEVEL"
echo ""

# Build
cd "$ROOT_DIR"
if [[ "$BUILD_MODE" == "release" ]]; then
    echo "→ Building exit service (release)..."
    cargo build -p blindhop-exit --release 2>&1
    BINARY="./target/release/blindhop-exit"
else
    echo "→ Building exit service (debug)..."
    cargo build -p blindhop-exit 2>&1
    BINARY="./target/debug/blindhop-exit"
fi

echo ""
echo "→ Starting exit service..."
echo "  (Nym address will be printed below and saved to .exit_nym_address)"
echo ""

RUST_LOG="$LOG_LEVEL" exec "$BINARY" --target-rpc "$TARGET_RPC"
