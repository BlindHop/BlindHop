#!/usr/bin/env bash
# Back up the BlindHop exit's keys and state (its Nym address).
#
#   sudo ./deploy/backup-exit-keys.sh [output-dir]
#
# Stops the exit for a few seconds so its databases are copied consistently,
# then starts it again. The archive contains private keys: it's created with
# mode 0600; keep it somewhere safe, off this server.
set -euo pipefail

SERVICE=blindhop-exit
STATE_DIR=/var/lib/blindhop-exit
OUT_DIR="${1:-.}"

[[ $EUID -eq 0 ]] || { echo "Error: run as root (sudo $0)" >&2; exit 1; }
[[ -d "$STATE_DIR/keys" ]] || { echo "Error: no keys in $STATE_DIR/keys" >&2; exit 1; }

archive="$OUT_DIR/blindhop-exit-keys-$(date -u +%Y%m%dT%H%M%SZ).tar.gz"

was_active=false
if systemctl is-active --quiet "$SERVICE"; then
    was_active=true
    systemctl stop "$SERVICE"
fi
# Restart the exit even if archiving fails.
trap '$was_active && systemctl start "$SERVICE"' EXIT

umask 077
tar -czf "$archive" -C "$STATE_DIR" keys .exit_nym_address
if [[ -n "${SUDO_USER:-}" ]]; then
    chown "$SUDO_USER" "$archive"
fi

echo "Backed up to $archive"
echo "Nym address in this backup: $(cat "$STATE_DIR/.exit_nym_address")"
echo "Copy it off the server, e.g.: scp <server>:$(realpath "$archive") ."
