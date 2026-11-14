#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
RULE=/etc/udev/rules.d/70-gsp370.rules

if [[ $EUID -ne 0 ]]; then
    echo "error: run this script as root (pkexec or sudo)" >&2
    exit 1
fi

install -Dm644 "$ROOT/packaging/70-gsp370.rules" "$RULE"
udevadm control --reload-rules

udevadm trigger --subsystem-match=tty

echo "Installed $RULE and reloaded udev."
echo "Unplug and replug the dongle once."
