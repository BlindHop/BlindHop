#!/usr/bin/env bash
set -euo pipefail

echo "=== BlindHop MVP Demo ==="
echo ""

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

# Build in release mode
echo "Building BlindHop..."
cargo build --workspace --release 2>&1 | tail -3

KEY_DIR="$PROJECT_DIR/target/demo-keys"
mkdir -p "$KEY_DIR"

# Generate relay keys (and store public keys)
echo ""
echo "Generating relay keys..."
for i in 1 2 3; do
    if [ ! -f "$KEY_DIR/relay${i}.key" ]; then
        PUB=$(cargo run -p blindhop-relay --release -- generate-key --output "$KEY_DIR/relay${i}.key" 2>/dev/null | tail -1)
        echo "$PUB" > "$KEY_DIR/relay${i}.pub"
        echo "  Relay $i key generated (pubkey: ${PUB:0:16}...)"
    else
        echo "  Relay $i key exists (reusing)"
    fi
done

# Read public keys for proxy config
RELAY1_PUB=$(cat "$KEY_DIR/relay1.pub")
RELAY2_PUB=$(cat "$KEY_DIR/relay2.pub")
RELAY3_PUB=$(cat "$KEY_DIR/relay3.pub")

# Generate a dummy destination key (the exit relay handles forwarding)
DEST_KEY="0000000000000000000000000000000000000000000000000000000000000000"

echo ""
echo "Starting relay nodes..."

# Start 3 relay nodes
cargo run -p blindhop-relay --release -- run \
    --listen 0.0.0.0:9401 \
    --secret-key-file "$KEY_DIR/relay1.key" \
    --hop-index 0 &
PID1=$!
echo "  Relay 1 (hop 0): PID $PID1, port 9401"

cargo run -p blindhop-relay --release -- run \
    --listen 0.0.0.0:9402 \
    --secret-key-file "$KEY_DIR/relay2.key" \
    --hop-index 1 &
PID2=$!
echo "  Relay 2 (hop 1): PID $PID2, port 9402"

cargo run -p blindhop-relay --release -- run \
    --listen 0.0.0.0:9403 \
    --secret-key-file "$KEY_DIR/relay3.key" \
    --hop-index 2 \
    --target "wss://sys.turboflakes.io/asset-hub-paseo" &
PID3=$!
echo "  Relay 3 (exit, hop 2): PID $PID3, port 9403"

sleep 2

# Start the proxy
echo ""
echo "Starting BlindHop proxy..."
cargo run -p blindhop-proxy --release -- \
    --listen 127.0.0.1:9500 \
    --relay-path "ws://127.0.0.1:9401,ws://127.0.0.1:9402,ws://127.0.0.1:9403" \
    --relay-keys "$RELAY1_PUB,$RELAY2_PUB,$RELAY3_PUB" \
    --target "wss://sys.turboflakes.io/asset-hub-paseo" \
    --target-key "$DEST_KEY" &
PID4=$!
echo "  Proxy: PID $PID4, port 9500"

sleep 1

echo ""
echo "Starting demo web server..."
echo "============================================"
echo "  Open http://localhost:8080 in your browser"
echo "============================================"
echo ""
echo "Press Ctrl+C to stop all processes."

# Serve the demo page
python3 -m http.server 8080 -d "$PROJECT_DIR/demo/" &
PID5=$!

# Cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    kill $PID1 $PID2 $PID3 $PID4 $PID5 2>/dev/null || true
    wait 2>/dev/null || true
    echo "Done."
}
trap cleanup EXIT INT TERM

# Wait for any process to exit
wait
