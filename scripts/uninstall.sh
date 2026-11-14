#!/usr/bin/env bash
set -euo pipefail
umask 022

PREFIX="${PREFIX:-/usr/local}"
MANIFEST="$PREFIX/share/gsp370-tray/install-manifest.txt"
if [[ "$PREFIX" == "/usr" || "$PREFIX" == "/usr/local" ]]; then
    RULE=/etc/udev/rules.d/70-gsp370.rules
else
    RULE="$PREFIX/lib/udev/rules.d/70-gsp370.rules"
fi
CONFIG_HOME="${XDG_CONFIG_HOME:-${HOME:-.}/.config}"
AUTOSTART="$CONFIG_HOME/autostart/gsp370-tray.desktop"
CONFIG="$CONFIG_HOME/gsp370-tray/config"

usage() {
    cat <<EOF
Usage: $0

Removes gsp370-tray using the manifest written by install.sh.  Run it as your
normal user; sudo is used only for files in system locations.

Environment:
  PREFIX        install prefix used at install time (default: /usr/local)

The user config at $CONFIG is kept.
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    exit 0
fi
if (($# > 0)); then
    echo "error: unknown argument: $1" >&2
    usage >&2
    exit 1
fi
if [[ "$EUID" -eq 0 ]]; then
    echo "error: run this script as your normal user, not with sudo or as root." >&2
    echo "It will ask for sudo only when removing system files." >&2
    exit 1
fi

remove_path() {
    local path="$1"
    local parent

    if [[ ! -e "$path" && ! -L "$path" ]]; then
        return 0
    fi

    parent="$(dirname -- "$path")"
    if [[ -w "$parent" ]]; then
        rm -f -- "$path"
    else
        sudo rm -f -- "$path"
    fi
    printf 'Removed %s\n' "$path"
}

fallback_manifest() {
    local output="$1"

    {
        printf '%s\n' \
            "$PREFIX/bin/gsp370-tray" \
            "$PREFIX/share/applications/gsp370-tray.desktop" \
            "$PREFIX/share/icons/hicolor/scalable/apps/gsp370-tray.svg" \
            "$PREFIX/share/licenses/gsp370-tray/NotoSans-OFL.txt"
        if [[ -d "$PREFIX/share/doc/gsp370-tray" ]]; then
            find "$PREFIX/share/doc/gsp370-tray" -type f -print
        fi
        if [[ -d "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery" ]]; then
            find "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery" -type f -print
        fi
        if [[ -f "$RULE" ]]; then
            printf '%s\n' "$RULE"
        fi
    } | sort -u >"$output"
}

if pkill -x gsp370-tray 2>/dev/null; then
    echo "Stopped the running gsp370-tray process."
fi

manifest_tmp="$(mktemp)"
trap 'rm -f "${manifest_tmp:-}"' EXIT

if [[ -f "$MANIFEST" ]]; then
    cat "$MANIFEST" >"$manifest_tmp"
elif sudo test -f "$MANIFEST"; then
    sudo cp -- "$MANIFEST" "$manifest_tmp"
else
    echo "warning: $MANIFEST not found; using the standard install paths." >&2
    fallback_manifest "$manifest_tmp"
fi

while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    case "$path" in
        "$RULE" | "$PREFIX"/*) remove_path "$path" ;;
        *)
            echo "warning: refusing to remove unexpected manifest path: $path" >&2
            ;;
    esac
done <"$manifest_tmp"

remove_path "$MANIFEST"

if [[ -f "$AUTOSTART" || -L "$AUTOSTART" ]]; then
    rm -f -- "$AUTOSTART"
    printf 'Removed %s\n' "$AUTOSTART"
fi

for dir in \
    "$PREFIX/share/doc/gsp370-tray/docs/screenshots" \
    "$PREFIX/share/doc/gsp370-tray/docs" \
    "$PREFIX/share/doc/gsp370-tray" \
    "$PREFIX/share/licenses/gsp370-tray" \
    "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery/contents/ui" \
    "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery/contents" \
    "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery" \
    "$PREFIX/share/gsp370-tray"; do
    [[ -d "$dir" ]] || continue
    if [[ -w "$(dirname -- "$dir")" ]]; then
        rmdir "$dir" 2>/dev/null || true
    else
        sudo rmdir "$dir" 2>/dev/null || true
    fi
done

if command -v udevadm >/dev/null 2>&1; then
    sudo udevadm control --reload-rules || true
    sudo udevadm trigger --subsystem-match=tty || true
fi

echo
echo "Uninstalled gsp370-tray from $PREFIX."
echo "Kept user config: $CONFIG"
echo "Kept user state/cache directories; they are temporary and can be removed manually."
