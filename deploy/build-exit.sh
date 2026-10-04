#!/usr/bin/env bash
# Build the BlindHop exit on the server, as a normal (non-root) user.
#
#   ./deploy/build-exit.sh
#
# Builds with --locked, so the exact dependency versions in Cargo.lock are
# used (the ones that were tested and audited). Then run install-exit.sh.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIN_RUST="1.88"  # let-chains in common/src/rpc.rs (nym-sdk itself needs 1.87)

die() { echo "Error: $*" >&2; exit 1; }

if [[ $EUID -eq 0 ]]; then
    die "run this as a normal user, not root (install-exit.sh is the root step)"
fi

# --- System packages ---
missing=()
command -v cc >/dev/null || missing+=(build-essential)
command -v pkg-config >/dev/null || missing+=(pkg-config)
if command -v pkg-config >/dev/null && ! pkg-config --exists openssl; then
    missing+=(libssl-dev)
fi
command -v git >/dev/null || missing+=(git)
if (( ${#missing[@]} )); then
    die "missing packages. Install them with:
  sudo apt-get update && sudo apt-get install -y ${missing[*]}"
fi

# --- Rust toolchain ---
if ! command -v cargo >/dev/null; then
    die "Rust is not installed. Install it (official installer) with:
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  source \"\$HOME/.cargo/env\""
fi
rust_version="$(rustc --version | awk '{print $2}')"
if [[ "$(printf '%s\n%s\n' "$MIN_RUST" "$rust_version" | sort -V | head -1)" != "$MIN_RUST" ]]; then
    die "Rust $rust_version is too old (need >= $MIN_RUST). Update with: rustup update stable"
fi

# --- Memory check: building nym-sdk needs a few GB of RAM ---
mem_kb=$(awk '/MemTotal/{print $2}' /proc/meminfo)
swap_kb=$(awk '/SwapTotal/{print $2}' /proc/meminfo)
if (( (mem_kb + swap_kb) < 4 * 1024 * 1024 )); then
    echo "Warning: RAM + swap is $(( (mem_kb + swap_kb) / 1024 )) MB; the build may run out of memory."
    echo "  To add a 4 GB swap file:"
    echo "    sudo fallocate -l 4G /swapfile && sudo chmod 600 /swapfile"
    echo "    sudo mkswap /swapfile && sudo swapon /swapfile"
    echo "  Or limit parallel jobs: CARGO_BUILD_JOBS=2 $0"
    echo
fi

cd "$REPO_DIR"
echo "Building blindhop-exit $(git describe --always --dirty 2>/dev/null || echo '') with Rust $rust_version ($(uname -m))..."
cargo build --release --locked -p blindhop-exit

BIN="$REPO_DIR/target/release/blindhop-exit"
echo
echo "Built: $BIN"
echo "  sha256: $(sha256sum "$BIN" | awk '{print $1}')"
echo
echo "Next: sudo ./deploy/install-exit.sh --target-rpc <wss://your-full-node>"
