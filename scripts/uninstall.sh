#!/usr/bin/env bash
set -euo pipefail

PREFIX="${PREFIX:-/usr/local}"
RULE=/etc/udev/rules.d/70-gsp370.rules

if [[ $EUID -ne 0 ]]; then
    exec sudo --preserve-env=PREFIX -- "$0" "$@"
fi

rm -f \
    "$PREFIX/bin/gsp370-tray" \
    "$PREFIX/share/applications/gsp370-tray.desktop" \
    "$PREFIX/share/icons/hicolor/scalable/apps/gsp370-tray.svg" \
    "$RULE"
rm -rf \
    "$PREFIX/share/doc/gsp370-tray" \
    "$PREFIX/share/gsp370-tray"

if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules
    udevadm trigger --subsystem-match=tty --action=change
fi

echo "Removed the application and udev rule."
echo "Your settings and autostart entry were left in place:"
echo "  ~/.config/gsp370-tray/gsp370-tray.conf"
echo "  ~/.config/autostart/gsp370-tray.desktop"
