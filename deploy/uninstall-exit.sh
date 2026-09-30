#!/usr/bin/env bash
# Remove the BlindHop exit service.
#
#   sudo ./deploy/uninstall-exit.sh           # keeps keys in /var/lib/blindhop-exit
#   sudo ./deploy/uninstall-exit.sh --purge   # also deletes keys and the service user
#
# Without --purge, reinstalling later brings the exit back with the same
# Nym address.
set -euo pipefail

SERVICE=blindhop-exit
PURGE=false
[[ "${1:-}" == "--purge" ]] && PURGE=true

[[ $EUID -eq 0 ]] || { echo "Error: run as root (sudo $0)" >&2; exit 1; }

systemctl disable --now "$SERVICE" 2>/dev/null || true
rm -f "/etc/systemd/system/$SERVICE.service" /usr/local/bin/blindhop-exit /etc/blindhop-exit.env
systemctl daemon-reload
echo "Removed the $SERVICE service, binary and config."

if $PURGE; then
    rm -rf /var/lib/blindhop-exit
    userdel blindhop 2>/dev/null || true
    echo "Deleted /var/lib/blindhop-exit (keys) and the blindhop user. The old Nym address is gone."
else
    echo "Kept /var/lib/blindhop-exit (keys); reinstalling restores the same Nym address."
fi
