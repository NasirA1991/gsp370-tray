# Installation

GSP 370 Tray runs on Linux and needs a StatusNotifierItem tray host. The
project is developed and tested on KDE Plasma 6 with CachyOS; other compliant
tray hosts may work but have not been tested. The application is Linux-only.

There are three supported installation paths: the Arch User Repository (AUR),
a prebuilt release archive, and a source build.

## Arch Linux / CachyOS (AUR)

The AUR package is the recommended installation on Arch Linux and derivatives:

```bash
paru -S gsp370-tray
# or
yay -S gsp370-tray
```

The package installs:

- `/usr/bin/gsp370-tray`
- the desktop entry and scalable icon
- the udev rule at `/usr/lib/udev/rules.d/70-gsp370.rules`
- the optional KDE Plasma widget
- the README, user documentation (including screenshots), contributor guide,
  security policy, changelog and licences under
  `/usr/share/doc/gsp370-tray/` and `/usr/share/licenses/gsp370-tray/`

The package's install hook reloads udev and re-adds the dongle's tty node. If
the tray still reports a permission error, unplug and replug the dongle once.

The AUR package is built from the tagged source archive published with each
GitHub release. The release workflow updates the AUR package automatically;
the package page is at
<https://aur.archlinux.org/packages/gsp370-tray>.

## Prebuilt release archive

Download `gsp370-tray-<version>-x86_64.tar.gz` and `SHA256SUMS` from the
[latest release](https://github.com/NasirA1991/gsp370-tray/releases/latest).

```bash
sha256sum -c --ignore-missing SHA256SUMS
tar -xzf gsp370-tray-<version>-x86_64.tar.gz
cd gsp370-tray-<version>
./install.sh
```

The archive contains the binary, `install.sh`, `uninstall.sh`, desktop entry,
icon, udev rule, Plasma widget, README, contributor guide, security policy,
user documentation (including screenshots), changelog and licences. The
installer installs into `/usr/local` by default, copies the documentation and
writes an install manifest. `INSTALL.txt` is a short pointer to these steps.

The published v2.2.2 archive predates this installer; use its manual
`INSTALL.txt`. Archives produced from this revision contain both scripts.

The prebuilt binary targets x86_64 Linux and needs glibc and a running D-Bus
session. Low-battery notifications additionally need `notify-send` from
`libnotify`. The Plasma widget is optional and requires Plasma 6.

## Build from source

### Requirements

- Rust 1.89 or newer (`rust-version` in `rust/Cargo.toml`)
- a C linker such as `gcc` or `clang`
- `git` to clone the repository
- `make` for the convenience targets (optional)
- Python 3 for the pseudo-terminal tests (optional)
- `sudo` for the system-wide install step

On Arch Linux, the build dependencies are in `base-devel` and `rust`:

```bash
sudo pacman -S --needed base-devel rust git
```

### Build and install

```bash
git clone https://github.com/NasirA1991/gsp370-tray.git
cd gsp370-tray
./scripts/install.sh
```

The script builds the release binary itself. To compile only, run
`make build` (or the equivalent Cargo command).

`scripts/install.sh` builds as your user, checks for Cargo/Rust 1.89+ and a C
linker, then re-runs itself through sudo to install the binary, desktop entry,
icon, Plasma widget (only when Plasma 6 is present), documentation, manifest
and udev rule. It installs under `/usr/local` by default.

Useful variants:

```bash
PREFIX=/usr ./scripts/install.sh              # install under /usr
sudo ./scripts/install.sh --system            # install an already-built binary
./scripts/install.sh --skip-deps              # bypass the dependency check
./scripts/install.sh --force                  # allow an install next to the AUR package
./scripts/install.sh --help                   # show the script's usage
```

Running the script with `sudo` from the start is unnecessary: the no-argument
form deliberately builds as the invoking user and escalates only for the
install step. `--system` is for an existing build and must run as root. The
installer refuses to run as root without `--system`.

### Source install paths

The source installer uses these paths by default:

| File | Destination |
|---|---|
| binary | `/usr/local/bin/gsp370-tray` |
| desktop entry | `/usr/local/share/applications/gsp370-tray.desktop` |
| icon | `/usr/local/share/icons/hicolor/scalable/apps/gsp370-tray.svg` |
| Plasma widget | `/usr/local/share/plasma/plasmoids/org.nasir.gsp370battery/` |
| documentation | `/usr/local/share/doc/gsp370-tray/` |
| font licence | `/usr/local/share/licenses/gsp370-tray/` |
| install manifest | `/usr/local/share/gsp370-tray/install-manifest.txt` |
| udev rule | `/etc/udev/rules.d/70-gsp370.rules` |

The AUR package uses the distribution paths under `/usr` and
`/usr/lib/udev/rules.d`.

For a source install under `/usr` or `/usr/local`, the rule is written to
`/etc/udev/rules.d/70-gsp370.rules`. For any other `PREFIX`, it is written to
`$PREFIX/lib/udev/rules.d/70-gsp370.rules`; udev does not read that path, so the
installer warns and you should use **Fix device permissions…** or install the
rule under `/etc` manually.

### Run the tests

Tests do not need the real headset. The pseudo-terminal suite uses a fake
dongle:

```bash
make test
make e2e
```

If `make` is unavailable, run the same checks directly:

```bash
cargo test --locked --manifest-path rust/Cargo.toml
cargo build --release --locked --manifest-path rust/Cargo.toml
python3 tests/e2e_pty.py ./rust/target/release/gsp370-tray
```

`make check` runs formatting, clippy, unit tests and the PTY suite. Note that
`make fmt` writes formatting changes; CI uses `cargo fmt --check` instead. See
[Contributing](../CONTRIBUTING.md) for the development workflow.

## Dongle permissions

The app opens the GSP 370 dongle's CDC-ACM serial node, normally
`/dev/ttyACM0`, at 921600 baud. The single supplied rule matches only
`SUBSYSTEM=="tty"` with USB IDs `1395:009a`:

- tags the device with `uaccess`, so the active logind seat user can open it
  without joining a group;
- deliberately does not set `GROUP` or `MODE`, because the default tty group
  differs between distributions;
- sets `ID_MM_DEVICE_IGNORE` so ModemManager does not probe the port.

The AUR package installs the rule automatically. A source install via
`scripts/install.sh` also installs it. The installer detects the
distribution's tty group for non-logind guidance: Arch normally uses `uucp`;
Debian, Ubuntu, Fedora and openSUSE normally use `dialout`.

When the tray reports `permission`, **Fix device permissions…** installs the
rule through one `pkexec` prompt. If `pkexec` is unavailable, the menu and
notification show the manual commands below.

For a manual install from a source checkout, or to repair permissions:

```bash
# From a source checkout:
sudo install -Dm644 packaging/70-gsp370.rules /etc/udev/rules.d/70-gsp370.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=tty
```

An AUR install already has the rule at `/usr/lib/udev/rules.d/70-gsp370.rules`;
reinstall the package instead of copying from a source tree. Replug the dongle
after installing the rule so the seat ACL is applied.

If your system does not apply logind seat ACLs, find the available group and
add your user to it:

```bash
getent group uucp    # Arch usually
getent group dialout # Debian/Ubuntu/Fedora/openSUSE usually
sudo usermod -aG dialout "$USER"  # use uucp on Arch if that is the group
```

Log out and back in, then check with `id -nG`. Do not hard-code a group without
checking `getent` on the target system. See
[Troubleshooting](troubleshooting.md#permission-denied) if the port still
cannot be opened.

## Uninstall

### AUR package

```bash
paru -Rns gsp370-tray
# or
sudo pacman -Rns gsp370-tray
```

The package removes its files and the udev rule. Your settings and autostart
entry are not package-owned, so they remain.

### Prebuilt archive

Run the uninstaller from the unpacked archive:

```bash
./uninstall.sh
```

The v2.2.2 archive predates `uninstall.sh`; remove its files using the manual
steps in that archive's `INSTALL.txt`.

### Source install

Run the uninstaller as your normal user; it asks for sudo only for system
files:

```bash
./scripts/uninstall.sh
```

It reads `/usr/local/share/gsp370-tray/install-manifest.txt`, removes exactly
the files listed there (or falls back to the standard paths if the manifest is
missing), removes the app's autostart entry, and deletes only now-empty
app-specific directories. It does not use `rm -rf` on directories it did not
create.

Your config is kept:

```text
~/.config/gsp370-tray/config
```

Remove it manually if you want a completely clean uninstall:

```bash
rm -rf ~/.config/gsp370-tray
```

The runtime status cache is temporary; it is removed by the session or can be
cleared with `rm -rf "$XDG_RUNTIME_DIR/gsp370-tray"` after quitting the app.
