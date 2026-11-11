# GSP 370 Tray

A small Linux tray app for the **Sennheiser/EPOS GSP 370** wireless headset.

It shows the headset battery percentage in the system tray, lets you set
sidetone from the tray menu, and warns when the battery is low.  No Qt, GTK or
audio server is required.

[![AUR](https://img.shields.io/aur/version/gsp370-tray?label=AUR)](https://aur.archlinux.org/packages/gsp370-tray)

![Battery percentage on dark and light panels](docs/tray-icon.png)

## Features

- Battery percentage in the tray and tooltip
- Transparent icon with light/dark adaptive text
- Low-battery alerts at 20 %, 10 % and 5 %
- Sidetone control from the tray menu
- Headset on/off detection and automatic reconnect
- Hides from the tray while the headset is off (KDE “only show when relevant”)
- Start-at-login toggle
- CLI for scripting: `--battery`, `--probe`, `--status`, `--set-sidetone`
- Small native binary; no web view or audio processing

**Not included:** equalizer/surround effects and firmware updates.  The EQ in
the Windows suite is host-side audio processing, not a GSP 370 setting, and
firmware flashing is deliberately unsupported.

### KDE Plasma panel widget

The package also includes an optional KDE Plasma 6 widget that shows the battery as
normal panel text (`84%`) in the system font, rather than as a 22 px tray icon.

Right-click the panel → **Enter Edit Mode** → **Add Widgets** → **GSP 370
Battery**.  The widget hides when the headset is off.

## Install

### Arch Linux / CachyOS (AUR)

```bash
yay -S gsp370-tray
# or
paru -S gsp370-tray
```

The package installs the udev rule for the dongle.  Replug the dongle once if
the tray reports a permission error.

### Release binary

Download the latest `gsp370-tray-*-x86_64.tar.gz` from
[GitHub Releases](https://github.com/NasirA1991/gsp370-tray/releases).  The
archive contains the binary, desktop file, icon, udev rule, the Plasma widget
and a short `INSTALL.txt`.

### Build from source

Requirements: Rust 1.89+ and a C linker.

```bash
git clone https://github.com/NasirA1991/gsp370-tray.git
cd gsp370-tray

cargo build --release --locked --manifest-path rust/Cargo.toml
python3 tests/e2e_pty.py ./rust/target/release/gsp370-tray

# or the same common tasks through make:
# make build
# make check

sudo ./scripts/install.sh
```

`scripts/install.sh` installs the binary under `/usr/local`, the desktop/icon
files, the Plasma widget, and the udev rule at
`/etc/udev/rules.d/70-gsp370.rules`.

## Usage

Run:

```bash
gsp370-tray
```

Click the tray icon for the menu.  Settings are stored in
`~/.config/gsp370-tray/config`.

CLI:

```bash
gsp370-tray --battery          # prints percentage, or "off" and exits 2
gsp370-tray --probe            # device path, link state and battery
gsp370-tray --status           # cached/probed state for the Plasma widget
gsp370-tray --set-sidetone 30  # 0 = off, 100 = maximum
gsp370-tray --device /dev/ttyACM1 --battery
```

`--device` also works when starting the tray in the normal no-action mode:
`gsp370-tray --device /dev/ttyACM1`.  The path must resolve under `/dev`
(symlinks such as `/dev/serial/by-id/...` are fine).

The sidetone level is not readable from the headset, so the menu shows the
last value set by this app.  Resetting the headset or changing sidetone from
another host does not update that stored value, so it can become stale.
Sidetone is only audible while the microphone is in use.

### Environment

- `GSP370_TTY=/dev/ttyACM1` — override the dongle device (useful for tests).
- `GSP370_TEXT_COLOR=R,G,B` — override the icon text colour, for unusual panel themes.

The Plasma widget calls `gsp370-tray --status` every minute. That command
reads the tray's status cache and only probes the serial port when the cache
is missing or stale.

## Troubleshooting

### Permission denied on `/dev/ttyACMn`

```bash
sudo install -Dm644 packaging/70-gsp370.rules /etc/udev/rules.d/70-gsp370.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add /sys/class/tty/ttyACMn  # use your dongle node
```

Replace `ttyACMn` with the node shown by `ls /dev/ttyACM*`.  Then replug the
dongle.  If your system does not apply logind seat ACLs, add your user to
`uucp` and log out/in:

```bash
sudo usermod -aG uucp "$USER"
```

### Dongle not found / headset off

```bash
lsusb -d 1395:009a
ls -l /dev/ttyACM*
```

The headset must be powered on and in range.  If the dongle is present but
the tray says the headset is off, that is the dongle reporting that it cannot
reach the headset.

### No tray icon

The app uses StatusNotifierItem.  Check that the desktop’s system tray is
running and that the app is not in the tray’s overflow menu.

## Bugs and requests

Please open an issue:

https://github.com/NasirA1991/gsp370-tray/issues

Include the output of:

```bash
gsp370-tray --probe
```

## Development

```bash
cargo test  --release --locked --manifest-path rust/Cargo.toml
cargo build --release --locked --manifest-path rust/Cargo.toml
python3 tests/e2e_pty.py ./rust/target/release/gsp370-tray
```

Protocol notes and reverse-engineering evidence are in
[docs/protocol.md](docs/protocol.md).

## Releasing

Push a tag matching the version in `rust/Cargo.toml`:

```bash
VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' rust/Cargo.toml | head -1)"
git tag "v$VERSION"
git push origin "v$VERSION"
```

The `Release` workflow runs the tests, builds the source/binary/Arch
artifacts, creates the GitHub release, and updates the AUR package.  It uses
the `AUR_SSH_PRIVATE_KEY` repository secret.

## License

MIT.  The embedded Noto Sans Medium digit subset is licensed under the SIL
Open Font License 1.1; see `rust/assets/NotoSans-OFL.txt`.

Not affiliated with or endorsed by EPOS, Sennheiser or Demant.
