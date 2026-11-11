#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_DIR="$ROOT/rust"
RUST_BIN="$RUST_DIR/target/release/gsp370-tray"
PREFIX="${PREFIX:-/usr/local}"
RULE=/etc/udev/rules.d/70-gsp370.rules

build() {
    if [[ -x "$RUST_BIN" ]]; then
        return
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: cargo is required to build the Rust app." >&2
        echo "Install Rust first, for example: sudo pacman -S rust" >&2
        echo "or: curl https://sh.rustup.rs -sSf | sh" >&2
        exit 1
    fi
    cargo build --release --locked --manifest-path "$RUST_DIR/Cargo.toml"
}

trigger_gsp_tty() {
    # A change event does not reliably re-run the seat ACL builtin.  Re-add
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

install_system() {
    if [[ ! -x "$RUST_BIN" ]]; then
        echo "error: $RUST_BIN not found; run $0 as your user first" >&2
        exit 1
    fi

    install -Dm755 "$RUST_BIN" "$PREFIX/bin/gsp370-tray"
    install -Dm644 "$ROOT/packaging/gsp370-tray.desktop" \
        "$PREFIX/share/applications/gsp370-tray.desktop"
    install -Dm644 "$ROOT/packaging/gsp370-tray.svg" \
        "$PREFIX/share/icons/hicolor/scalable/apps/gsp370-tray.svg"
    install -d "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery"
    cp -a "$ROOT/packaging/plasmoid/gsp370battery/." \
        "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery/"
    install -Dm644 "$ROOT/README.md" "$PREFIX/share/doc/gsp370-tray/README.md"
    install -Dm644 "$ROOT/CONTRIBUTING.md" "$PREFIX/share/doc/gsp370-tray/CONTRIBUTING.md"
    install -Dm644 "$ROOT/SECURITY.md" "$PREFIX/share/doc/gsp370-tray/SECURITY.md"
    install -Dm644 "$ROOT/CHANGELOG.md" "$PREFIX/share/doc/gsp370-tray/CHANGELOG.md"
    install -Dm644 "$ROOT/LICENSE" "$PREFIX/share/doc/gsp370-tray/LICENSE"
    for doc in "$ROOT"/docs/*.md; do
        install -Dm644 "$doc" "$PREFIX/share/doc/gsp370-tray/docs/$(basename "$doc")"
    done
    install -Dm644 "$ROOT/docs/tray-icon.png" "$PREFIX/share/doc/gsp370-tray/docs/tray-icon.png"
    install -Dm644 "$ROOT/rust/assets/NotoSans-OFL.txt" \
        "$PREFIX/share/licenses/gsp370-tray/NotoSans-OFL.txt"

    if [[ -d /etc/udev/rules.d ]]; then
        install -Dm644 "$ROOT/packaging/70-gsp370.rules" "$RULE"
        if command -v udevadm >/dev/null 2>&1; then
            udevadm control --reload-rules
            trigger_gsp_tty
        fi
    else
        echo "warning: /etc/udev/rules.d does not exist; install packaging/70-gsp370.rules manually" >&2
    fi

    echo
    echo "Installed gsp370-tray under $PREFIX."
    echo "The udev rule is at $RULE."
    echo "Start the tray from your application menu or with: gsp370-tray"
    echo "The KDE Plasma widget is available under Add Widgets → GSP 370 Battery."
    echo "If the port still cannot be opened, unplug/replug the dongle once."
}

case "${1:-}" in
    --system)
        if [[ $EUID -ne 0 ]]; then
            echo "error: --system must run as root" >&2
            exit 1
        fi
        install_system
        ;;
    --help|-h)
        cat <<EOF
Usage: $0 [--system]

Builds the Rust app and installs it system-wide.  Without --system it builds
as your user, then re-runs itself through sudo for the install step.

Environment:
  PREFIX  install prefix (default: /usr/local)
EOF
        ;;
    "")
        build
        if [[ $EUID -eq 0 ]]; then
            install_system
        elif command -v sudo >/dev/null 2>&1; then
            exec sudo --preserve-env=PREFIX -- "$0" --system
        else
            echo "error: build succeeded, but sudo is not available." >&2
            echo "Run this as root to install: $0 --system" >&2
            exit 1
        fi
        ;;
    *)
        echo "error: unknown argument: $1" >&2
        exit 1
        ;;
esac
