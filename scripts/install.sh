#!/usr/bin/env bash
set -euo pipefail
umask 022

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/../rust/Cargo.toml" ]]; then
    ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
else
    ROOT="$SCRIPT_DIR"
fi

RUST_MANIFEST="$ROOT/rust/Cargo.toml"
RUST_BIN="$ROOT/rust/target/release/gsp370-tray"
PREBUILT_BIN="$ROOT/gsp370-tray"
PREFIX="${PREFIX:-/usr/local}"
if [[ "$PREFIX" == "/usr" || "$PREFIX" == "/usr/local" ]]; then
    RULE=/etc/udev/rules.d/70-gsp370.rules
else
    RULE="$PREFIX/lib/udev/rules.d/70-gsp370.rules"
fi
MANIFEST="$PREFIX/share/gsp370-tray/install-manifest.txt"

SYSTEM=0
FORCE=0
SKIP_DEPS=0

usage() {
    cat <<EOF
Usage: $0 [--system] [--force] [--skip-deps]

Builds the Rust app and installs it system-wide.  Without --system it builds
as your user, then re-runs itself through sudo for the install step.

Options:
  --system      install an already-built/prebuilt binary; must run as root
  --force       install even if the AUR package gsp370-tray is installed
  --skip-deps   skip the Cargo/Rust/linker prerequisite checks
  -h, --help    show this help

Environment:
  PREFIX        install prefix (default: /usr/local)

The installer writes $PREFIX/share/gsp370-tray/install-manifest.txt.  Run
scripts/uninstall.sh as your normal user to remove the files listed there.
EOF
}

version_ge() {
    local have="$1"
    local want="$2"
    local have_major have_minor want_major want_minor rest

    IFS=. read -r have_major have_minor rest <<<"$have"
    IFS=. read -r want_major want_minor rest <<<"$want"
    [[ "$have_major" =~ ^[0-9]+$ && "$have_minor" =~ ^[0-9]+$ ]] || return 1
    ((have_major > want_major || (have_major == want_major && have_minor >= want_minor)))
}

distro_family() {
    local id="" like=""
    if [[ -r /etc/os-release ]]; then
        id="$(awk -F= '$1 == "ID" {gsub(/"/, "", $2); print $2; exit}' /etc/os-release)"
        like="$(awk -F= '$1 == "ID_LIKE" {gsub(/"/, "", $2); print $2; exit}' /etc/os-release)"
    fi
    case " ${id,,} ${like,,} " in
        *arch*) echo arch ;;
        *debian* | *ubuntu* | *linuxmint*) echo debian ;;
        *fedora* | *rhel* | *centos*) echo fedora ;;
        *suse*) echo suse ;;
        *) echo unknown ;;
    esac
}

install_command() {
    local family="$1"
    local packages="$2"
    case "$family" in
        arch) printf 'sudo pacman -S --needed %s' "$packages" ;;
        debian) printf 'sudo apt install %s' "$packages" ;;
        fedora) printf 'sudo dnf install %s' "$packages" ;;
        suse) printf 'sudo zypper install %s' "$packages" ;;
        *) printf 'install %s with your package manager' "$packages" ;;
    esac
}

build_packages() {
    case "$(distro_family)" in
        arch) echo 'rust base-devel' ;;
        debian) echo 'rustc cargo build-essential' ;;
        fedora) echo 'rust cargo gcc' ;;
        suse) echo 'rust cargo gcc' ;;
        *) echo 'Rust 1.89+ and a C linker (cc)' ;;
    esac
}

check_deps() {
    local family missing=0 cargo_version rustc_version
    family="$(distro_family)"

    if ! command -v cargo >/dev/null 2>&1; then
        echo "warning: cargo was not found" >&2
        missing=1
    fi
    if ! command -v rustc >/dev/null 2>&1; then
        echo "warning: rustc was not found" >&2
        missing=1
    fi
    if ! command -v cc >/dev/null 2>&1; then
        echo "warning: a C linker (cc) was not found" >&2
        missing=1
    fi

    if ((missing == 0)); then
        cargo_version="$(cargo --version | awk '{print $2}')"
        rustc_version="$(rustc --version | awk '{print $2}')"
        if ! version_ge "$cargo_version" 1.89; then
            echo "warning: cargo $cargo_version is older than 1.89" >&2
            missing=1
        fi
        if ! version_ge "$rustc_version" 1.89; then
            echo "warning: rustc $rustc_version is older than 1.89" >&2
            missing=1
        fi
    fi

    if ((missing != 0)); then
        echo >&2
        echo "Install Rust 1.89 or newer with rustup:" >&2
        echo "  curl https://sh.rustup.rs -sSf | sh" >&2
        echo "Or install the distribution packages:" >&2
        echo "  $(install_command "$family" "$(build_packages)")" >&2
        echo >&2
        echo "Use --skip-deps only if you have already verified these tools." >&2
        exit 1
    fi
}

asset_path() {
    local name="$1"
    local candidate
    for candidate in "$ROOT/packaging/$name" "$ROOT/$name"; do
        if [[ -e "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

plasmoid_source() {
    if [[ -d "$ROOT/packaging/plasmoid/gsp370battery" ]]; then
        printf '%s\n' "$ROOT/packaging/plasmoid/gsp370battery"
        return 0
    fi
    if [[ -d "$ROOT/gsp370battery" ]]; then
        printf '%s\n' "$ROOT/gsp370battery"
        return 0
    fi
    return 1
}

font_license_path() {
    if [[ -f "$ROOT/rust/assets/NotoSans-OFL.txt" ]]; then
        printf '%s\n' "$ROOT/rust/assets/NotoSans-OFL.txt"
        return 0
    fi
    if [[ -f "$ROOT/NotoSans-OFL.txt" ]]; then
        printf '%s\n' "$ROOT/NotoSans-OFL.txt"
        return 0
    fi
    return 1
}

find_binary() {
    if [[ -x "$RUST_BIN" ]]; then
        printf '%s\n' "$RUST_BIN"
        return 0
    fi
    if [[ -x "$PREBUILT_BIN" ]]; then
        printf '%s\n' "$PREBUILT_BIN"
        return 0
    fi
    return 1
}

has_plasma() {
    command -v plasmashell >/dev/null 2>&1 ||
        command -v kpackagetool6 >/dev/null 2>&1 ||
        [[ -d /usr/share/plasma/plasmoids ]]
}

detect_group() {
    local family first second
    family="$(distro_family)"
    case "$family" in
        arch) first="uucp"; second="dialout" ;;
        *) first="dialout"; second="uucp" ;;
    esac
    if getent group "$first" >/dev/null 2>&1; then
        echo "$first"
    elif getent group "$second" >/dev/null 2>&1; then
        echo "$second"
    fi
}

runtime_warnings() {
    local family pkg enabled group user
    family="$(distro_family)"
    user="${USER:-$(id -un)}"

    if ! command -v notify-send >/dev/null 2>&1; then
        case "$family" in
            arch) pkg=libnotify ;;
            debian) pkg=libnotify-bin ;;
            fedora) pkg=libnotify ;;
            suse) pkg=libnotify-tools ;;
            *) pkg=libnotify ;;
        esac
        echo "warning: notify-send is missing; low-battery notifications will be skipped." >&2
        echo "  $(install_command "$family" "$pkg")" >&2
    fi

    if [[ "${XDG_CURRENT_DESKTOP:-}" == *GNOME* ]]; then
        enabled=""
        if command -v gnome-extensions >/dev/null 2>&1; then
            enabled="$(gnome-extensions list --enabled 2>/dev/null || true)"
        fi
        if ! grep -qi 'appindicator' <<<"$enabled"; then
            echo "warning: GNOME needs the AppIndicator and KStatusNotifierItem Support extension." >&2
            case "$family" in
                suse)
                    echo "  openSUSE package name is unverified; install the AppIndicator extension." >&2
                    ;;
                arch | debian | fedora)
                    echo "  $(install_command "$family" 'gnome-shell-extension-appindicator')" >&2
                    ;;
                *)
                    echo "  Install gnome-shell-extension-appindicator with your package manager." >&2
                    ;;
            esac
        fi
    fi

    if command -v pgrep >/dev/null 2>&1 && pgrep -x ModemManager >/dev/null 2>&1; then
        echo "note: ModemManager is running; the udev rule marks the dongle so it is not probed." >&2
    elif command -v systemctl >/dev/null 2>&1 && systemctl is-active --quiet ModemManager 2>/dev/null; then
        echo "note: ModemManager is running; the udev rule marks the dongle so it is not probed." >&2
    fi

    if ! command -v loginctl >/dev/null 2>&1 ||
        ! systemctl is-active --quiet systemd-logind 2>/dev/null; then
        group="$(detect_group)"
        echo "warning: systemd-logind is not active; the uaccess fallback will not apply." >&2
        if [[ -n "$group" ]]; then
            echo "  Add your user to the $group group, then log out and back in:" >&2
            echo "    sudo usermod -aG $group \"$user\"" >&2
        else
            echo "  No uucp or dialout group was found; check your distribution's tty group." >&2
        fi
    fi
}

build() {
    if [[ -f "$RUST_MANIFEST" ]]; then
        if ((SKIP_DEPS == 0)); then
            check_deps
        fi
        cargo build --release --locked --manifest-path "$RUST_MANIFEST"
        return
    fi

    if [[ -x "$PREBUILT_BIN" ]]; then
        echo "Using prebuilt binary: $PREBUILT_BIN"
        return
    fi

    echo "error: no Cargo.toml or prebuilt gsp370-tray binary found in $ROOT" >&2
    echo "Run this script from a source checkout or an unpacked release archive." >&2
    exit 1
}

verify_install_modes() {
    local root dir mode

    for root in "$PREFIX/share/plasma" "$PREFIX/share/gsp370-tray"; do
        [[ -d "$root" ]] || continue
        while IFS= read -r -d '' dir; do
            mode="$(stat -c '%a' -- "$dir" 2>/dev/null || true)"
            if [[ "$mode" != "755" ]]; then
                echo "error: installed directory has mode ${mode:-unknown}: $dir" >&2
                echo "Expected mode 755 for the Plasma widget and installed manifest." >&2
                echo "Reinstall with a 022 umask, or fix the mode manually." >&2
                return 1
            fi
        done < <(find "$root" -type d -print0)
    done
}

install_system() {
    local binary desktop icon source_rule font_license widget
    local installed_widget=0 rule_installed=0 manifest_tmp

    binary="$(find_binary)" || {
        echo "error: no built or prebuilt gsp370-tray binary found in $ROOT" >&2
        echo "Run the installer as your user first, or use --system after building." >&2
        exit 1
    }

    if command -v pacman >/dev/null 2>&1 && pacman -Qq gsp370-tray >/dev/null 2>&1; then
        if ((FORCE == 0)); then
            echo "error: the AUR package gsp370-tray is installed." >&2
            echo "Remove it first, or re-run with --force to keep two installations." >&2
            exit 1
        fi
        echo "warning: installing alongside the AUR package because --force was given." >&2
    fi

    desktop="$(asset_path gsp370-tray.desktop)" || {
        echo "error: gsp370-tray.desktop not found" >&2
        exit 1
    }
    icon="$(asset_path gsp370-tray.svg)" || {
        echo "error: gsp370-tray.svg not found" >&2
        exit 1
    }

    install -Dm755 "$binary" "$PREFIX/bin/gsp370-tray"
    install -Dm644 "$desktop" "$PREFIX/share/applications/gsp370-tray.desktop"
    install -Dm644 "$icon" "$PREFIX/share/icons/hicolor/scalable/apps/gsp370-tray.svg"

    if has_plasma; then
        widget="$(plasmoid_source)" || {
            echo "error: the Plasma widget files were not found" >&2
            exit 1
        }
        install -d "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery"
        cp -a "$widget/." "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery/"
        installed_widget=1
    else
        echo "note: Plasma 6/plasma-workspace not found; skipping the optional panel widget." >&2
    fi

    install -Dm644 "$ROOT/README.md" "$PREFIX/share/doc/gsp370-tray/README.md"
    install -Dm644 "$ROOT/CONTRIBUTING.md" "$PREFIX/share/doc/gsp370-tray/CONTRIBUTING.md"
    install -Dm644 "$ROOT/SECURITY.md" "$PREFIX/share/doc/gsp370-tray/SECURITY.md"
    install -Dm644 "$ROOT/CHANGELOG.md" "$PREFIX/share/doc/gsp370-tray/CHANGELOG.md"
    install -Dm644 "$ROOT/LICENSE" "$PREFIX/share/doc/gsp370-tray/LICENSE"
    for doc in "$ROOT"/docs/*.md; do
        install -Dm644 "$doc" "$PREFIX/share/doc/gsp370-tray/docs/$(basename "$doc")"
    done
    if [[ -f "$ROOT/docs/tray-icon.png" ]]; then
        install -Dm644 "$ROOT/docs/tray-icon.png" \
            "$PREFIX/share/doc/gsp370-tray/docs/tray-icon.png"
    fi
    for shot in "$ROOT"/docs/screenshots/*.png; do
        [[ -e "$shot" ]] || continue
        install -Dm644 "$shot" \
            "$PREFIX/share/doc/gsp370-tray/docs/screenshots/$(basename "$shot")"
    done

    font_license="$(font_license_path)" || {
        echo "error: the Noto font licence was not found" >&2
        exit 1
    }
    install -Dm644 "$font_license" \
        "$PREFIX/share/licenses/gsp370-tray/NotoSans-OFL.txt"

    source_rule="$(asset_path 70-gsp370.rules)" || {
        echo "error: 70-gsp370.rules not found" >&2
        exit 1
    }
    install -Dm644 "$source_rule" "$RULE"
    rule_installed=1
    case "$RULE" in
        /etc/udev/rules.d/*)
            if command -v udevadm >/dev/null 2>&1; then
                if ! udevadm control --reload-rules ||
                    ! udevadm trigger --subsystem-match=tty; then
                    echo "warning: udev reload failed; replug the dongle after boot." >&2
                fi
            else
                echo "warning: udevadm not found; reload udev manually." >&2
            fi
            ;;
        *)
            echo "warning: udev does not read $PREFIX/lib/udev/rules.d." >&2
            echo "  Use \"Fix device permissions…\" in the tray menu, or run:" >&2
            echo "    sudo install -Dm644 packaging/70-gsp370.rules /etc/udev/rules.d/70-gsp370.rules" >&2
            ;;
    esac

    manifest_tmp="$(mktemp)"
    trap 'rm -f "${manifest_tmp:-}"' EXIT
    {
        printf '%s\n' "$PREFIX/bin/gsp370-tray"
        printf '%s\n' "$PREFIX/share/applications/gsp370-tray.desktop"
        printf '%s\n' "$PREFIX/share/icons/hicolor/scalable/apps/gsp370-tray.svg"
        find "$PREFIX/share/doc/gsp370-tray" -type f -print
        printf '%s\n' "$PREFIX/share/licenses/gsp370-tray/NotoSans-OFL.txt"
        if ((installed_widget == 1)); then
            find "$PREFIX/share/plasma/plasmoids/org.nasir.gsp370battery" -type f -print
        fi
        if ((rule_installed == 1)); then
            printf '%s\n' "$RULE"
        fi
    } | sort -u >"$manifest_tmp"
    install -Dm644 "$manifest_tmp" "$MANIFEST"
    verify_install_modes

    echo
    echo "Installed gsp370-tray under $PREFIX."
    echo "Install manifest: $MANIFEST"
    if [[ -x "$ROOT/scripts/uninstall.sh" ]]; then
        echo "Uninstall: $ROOT/scripts/uninstall.sh"
    elif [[ -x "$ROOT/uninstall.sh" ]]; then
        echo "Uninstall: $ROOT/uninstall.sh"
    fi
    echo "Start the tray from your application menu or with: gsp370-tray"
    if ((installed_widget == 1)); then
        echo "The KDE Plasma widget is available under Add Widgets → GSP 370 Battery."
    fi
    if ((rule_installed == 1)); then
        echo "The udev rule is at $RULE."
        if [[ "$RULE" == /etc/udev/rules.d/* ]]; then
            echo "Unplug and replug the dongle once so the new rule is applied."
        else
            echo "This prefix is not read by udev; see the warning above."
        fi
    fi
}

while (($# > 0)); do
    case "$1" in
        --system) SYSTEM=1 ;;
        --force) FORCE=1 ;;
        --skip-deps) SKIP_DEPS=1 ;;
        --help | -h)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
    shift
done

if ((SYSTEM == 1)); then
    if [[ "$EUID" -ne 0 ]]; then
        echo "error: --system must run as root" >&2
        exit 1
    fi
    install_system
    exit 0
fi

if [[ "$EUID" -eq 0 ]]; then
    echo "error: do not run this installer with sudo or as root." >&2
    echo "Run it as your normal user; it will ask for sudo for the system install step." >&2
    echo "For an already-built binary: sudo $0 --system" >&2
    exit 1
fi

build
runtime_warnings

if ! command -v sudo >/dev/null 2>&1; then
    echo "error: sudo is required for the system install step." >&2
    echo "Build succeeded; run as root: $0 --system" >&2
    exit 1
fi

sudo_args=(--preserve-env=PREFIX -- "$0" --system)
if ((FORCE == 1)); then
    sudo_args+=(--force)
fi
exec sudo "${sudo_args[@]}"
