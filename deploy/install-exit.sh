#!/usr/bin/env bash
# Install or upgrade the BlindHop exit as a systemd service.
#
#   First install:  sudo ./deploy/install-exit.sh --target-rpc wss://your-full-node
#   Upgrade:        sudo ./deploy/install-exit.sh
#   Change gateway: sudo ./deploy/install-exit.sh --gateway <identity key>
#
# Run build-exit.sh first. Re-running this script upgrades the binary and
# unit and restarts the service; the exit's keys (and so its Nym address)
# are kept in /var/lib/blindhop-exit. Settings not given are kept from the
# last install. Changing the gateway changes the part of the Nym address
# after the "@"; update the demo and proxy users afterwards.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE=blindhop-exit
SERVICE_USER=blindhop
BIN_SRC="$REPO_DIR/target/release/blindhop-exit"
BIN_DST=/usr/local/bin/blindhop-exit
ENV_FILE=/etc/blindhop-exit.env
UNIT_DST=/etc/systemd/system/$SERVICE.service
STATE_DIR=/var/lib/blindhop-exit
TARGET_RPC=""
GATEWAY=""

die() { echo "Error: $*" >&2; exit 1; }

usage() {
    sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
    exit "${1:-0}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target-rpc) TARGET_RPC="${2:-}"; shift 2 ;;
        --gateway)    GATEWAY="${2:-}"; shift 2 ;;
        --binary)     BIN_SRC="${2:-}"; shift 2 ;;
        -h|--help)    usage 0 ;;
        *)            echo "Unknown option: $1" >&2; usage 1 ;;
    esac
done

[[ $EUID -eq 0 ]] || die "run as root (sudo $0 ...)"
command -v systemctl >/dev/null || die "systemd is required"
[[ -x "$BIN_SRC" ]] || die "no binary at $BIN_SRC; run ./deploy/build-exit.sh first"

if [[ -n "$TARGET_RPC" ]]; then
    [[ "$TARGET_RPC" =~ ^wss?:// ]] || die "--target-rpc must be a ws:// or wss:// URL"
    [[ "$TARGET_RPC" =~ ^ws:// ]] && echo "Warning: $TARGET_RPC is unencrypted (ws://)."
elif [[ ! -f "$ENV_FILE" ]]; then
    die "first install needs --target-rpc <wss://your-full-node>"
fi
if [[ -n "$GATEWAY" && ! "$GATEWAY" =~ ^[1-9A-HJ-NP-Za-km-z]{32,44}$ ]]; then
    die "--gateway must be a gateway's base58 identity key"
fi

# --- Service user (no login, no home) ---
if ! id "$SERVICE_USER" >/dev/null 2>&1; then
    useradd --system --no-create-home --home-dir "$STATE_DIR" \
        --shell /usr/sbin/nologin "$SERVICE_USER"
    echo "Created system user $SERVICE_USER"
fi

# --- Configuration (settings not given keep their current values) ---
if [[ -n "$TARGET_RPC" || -n "$GATEWAY" ]]; then
    if [[ -f "$ENV_FILE" ]]; then
        [[ -n "$TARGET_RPC" ]] || TARGET_RPC="$(sed -n 's/^TARGET_RPC=//p' "$ENV_FILE")"
        [[ -n "$GATEWAY" ]] || GATEWAY="$(sed -n 's/^BLINDHOP_EXIT_GATEWAY=//p' "$ENV_FILE")"
    fi
    install -m 0644 /dev/null "$ENV_FILE"
    cat > "$ENV_FILE" <<EOF
# Substrate full node the BlindHop exit forwards to. It sees every query the
# exit forwards (but not who sent it), so use a node you trust.
TARGET_RPC=$TARGET_RPC
EOF
    if [[ -n "$GATEWAY" ]]; then
        cat >> "$ENV_FILE" <<EOF
# Nym gateway the exit connects through (identity key). The exit's Nym
# address ends in it. Remove this line to keep whichever gateway it has.
BLINDHOP_EXIT_GATEWAY=$GATEWAY
EOF
    fi
    echo "Wrote $ENV_FILE (TARGET_RPC=$TARGET_RPC${GATEWAY:+, gateway $GATEWAY})"
fi

# --- Binary (replaced atomically) and unit ---
install -m 0755 "$BIN_SRC" "$BIN_DST.new"
mv -f "$BIN_DST.new" "$BIN_DST"
install -m 0644 "$REPO_DIR/deploy/$SERVICE.service" "$UNIT_DST"
echo "Installed $BIN_DST ($(sha256sum "$BIN_DST" | awk '{print $1}'))"

systemctl daemon-reload
systemctl enable "$SERVICE" >/dev/null
# Restart (not start) so an upgrade picks up the new binary. SIGTERM makes
# the running exit disconnect cleanly first.
systemctl restart "$SERVICE"

# --- Wait for the exit to register on the Nym network ---
echo -n "Waiting for the exit to connect to the Nym network"
addr_file="$STATE_DIR/.exit_nym_address"
for _ in $(seq 1 60); do
    if ! systemctl is-active --quiet "$SERVICE"; then
        echo; echo "The service stopped. Recent log:" >&2
        journalctl -u "$SERVICE" -n 40 --no-pager >&2
        exit 1
    fi
    if [[ -s "$addr_file" && "$addr_file" -nt "$BIN_DST" ]]; then
        break
    fi
    echo -n "."; sleep 2
done
echo

if [[ ! -s "$addr_file" || ! "$addr_file" -nt "$BIN_DST" ]]; then
    echo "The exit hasn't reported its address yet. Check the log:" >&2
    echo "  journalctl -u $SERVICE -f" >&2
    exit 1
fi

cat <<EOF

BlindHop exit is running.

  Nym address: $(cat "$addr_file")

Put this address in demo/app.js (CONFIG.defaultExitAddress) and give it to
proxy users (--exit-address).

Back up the keys now; losing them changes the address:
  sudo $REPO_DIR/deploy/backup-exit-keys.sh

Status:  systemctl status $SERVICE
Logs:    journalctl -u $SERVICE -f
EOF
