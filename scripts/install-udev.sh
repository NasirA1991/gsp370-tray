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

trigger_gsp_tty() {
    # A "change" event does not re-run the seat ACL builtin reliably.  Re-add
    # just the GSP 370 tty node so 73-seat-late.rules applies uaccess to the
    # already-connected dongle.
    local tty dev
    for tty in /sys/class/tty/ttyACM*; do
        [[ -e "$tty/device" ]] || continue
        dev="$(readlink -f "$tty/device")"
        [[ "$(cat "$dev/../idVendor" 2>/dev/null)" == "1395" ]] || continue
        [[ "$(cat "$dev/../idProduct" 2>/dev/null)" == "009a" ]] || continue
        udevadm trigger --action=add "$tty"
    done
}

trigger_gsp_tty

echo "Installed $RULE and reloaded udev."
