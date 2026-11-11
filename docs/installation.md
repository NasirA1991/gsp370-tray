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
less INSTALL.txt
```

The archive is a manual-install bundle. It contains the binary, desktop entry,
icon, udev rule, Plasma widget, README, contributor guide, security policy,
user documentation (including screenshots), changelog and licences. Follow
`INSTALL.txt` for the exact commands; they install into `/usr/local` by
default. The documentation and licences remain in the unpacked archive; the
manual commands do not copy them into `/usr/local/share`.

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
cargo build --release --locked --manifest-path rust/Cargo.toml
./scripts/install.sh
```

The optional `make build` target runs the same command.

`scripts/install.sh` builds as your user if needed, then re-runs itself through
sudo to install the binary, desktop entry, icon, Plasma widget, documentation
and udev rule. It installs under `/usr/local` by default.

Useful variants:

```bash
PREFIX=/usr ./scripts/install.sh              # install under /usr
sudo ./scripts/install.sh --system            # install an already-built binary
./scripts/install.sh --help                   # show the script's usage
```

Running the script with `sudo` from the start is unnecessary: the no-argument
form deliberately builds as the invoking user and escalates only for the
install step. `--system` is for an existing build and must run as root.

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
| udev rule | `/etc/udev/rules.d/70-gsp370.rules` |

The AUR package uses the distribution paths under `/usr` and
`/usr/lib/udev/rules.d`.

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

Or without `make`:

```bash
cargo test --locked --manifest-path rust/Cargo.toml
python3 tests/e2e_pty.py ./rust/target/release/gsp370-tray
```

`make check` runs formatting, clippy, unit tests and the PTY suite. Note that
`make fmt` writes formatting changes; CI uses `cargo fmt --check` instead. See
[Contributing](../CONTRIBUTING.md) for the development workflow.

## Dongle permissions

The app opens the GSP 370 dongle's CDC-ACM serial node, normally
`/dev/ttyACM0`, at 921600 baud. The supplied udev rule:

- tags the device with `uaccess`, so the active logind seat user can open it
  without joining a group;
- sets group `uucp` and mode `0660` as a fallback for systems without logind;
- tells ModemManager not to probe the port.

The AUR package installs the rule automatically. A source install via
`scripts/install.sh` also installs it. For a manual install from a source
checkout, or to repair permissions:

```bash
# From a source checkout:
sudo install -Dm644 packaging/70-gsp370.rules /etc/udev/rules.d/70-gsp370.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add /sys/class/tty/ttyACM0
```

Replace `ttyACM0` with the node for the GSP 370 dongle. An AUR install already
has the rule at `/usr/lib/udev/rules.d/70-gsp370.rules`; reinstall the package
instead of copying from a source tree. Replug the dongle after installing the
rule so the seat ACL is applied.

If your system does not apply logind seat ACLs, add your user to `uucp` and log
out and back in:

```bash
sudo usermod -aG uucp "$USER"
```

Check the result with `id -nG`. See
[Troubleshooting](troubleshooting.md#permission-denied) if the port still
cannot be opened.

## Uninstall

### AUR package

```bash
yay -Rns gsp370-tray
# or
sudo pacman -Rns gsp370-tray
```

The package removes its files and the udev rule. Your settings and autostart
entry are not package-owned, so they remain.

### Source install

```bash
sudo ./scripts/uninstall.sh
```

The script removes the installed binary, desktop entry, icon, documentation,
font licence, Plasma widget and udev rule. It deliberately leaves your settings
and autostart entry:

```text
~/.config/gsp370-tray/config
~/.config/autostart/gsp370-tray.desktop
```

Remove those manually if you want a completely clean uninstall:

```bash
rm -rf ~/.config/gsp370-tray
rm -f ~/.config/autostart/gsp370-tray.desktop
```

The runtime status cache is temporary; it is removed by the session or can be
cleared with `rm -rf "$XDG_RUNTIME_DIR/gsp370-tray"` after quitting the app.
