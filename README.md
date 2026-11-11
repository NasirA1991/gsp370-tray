# GSP 370 Tray

A small Linux tray application for the **Sennheiser/EPOS GSP 370** wireless
headset. It shows the headset battery percentage, lets you set sidetone from
the tray menu, and warns when the battery is low.

The app talks to the GSP 370 dongle directly over its CDC-ACM serial interface.
It does not need Qt, GTK, a web view, or an audio server.

[![AUR version](https://img.shields.io/aur/version/gsp370-tray?label=AUR)](https://aur.archlinux.org/packages/gsp370-tray)

![Battery percentage rendered for dark and light panels](docs/tray-icon.png)

## Features

- Battery percentage in the tray icon and tooltip
- Transparent icon with text that adapts to light and dark panels
- Low-battery notifications at 20 %, 10 % and 5 %
- Sidetone control from the tray menu, or from the CLI
- Headset on/off detection and automatic reconnection
- Optional KDE Plasma 6 panel widget that renders the percentage as native
  panel text
- Start-at-login toggle
- Scriptable CLI: `--battery`, `--probe`, `--status`, `--set-sidetone`
- Small native binary with no audio processing

**Not included:** equalizer/surround effects and firmware updates. The EQ in
the Windows suite is host-side audio processing, not a GSP 370 setting, and
firmware flashing is deliberately unsupported.

## Requirements

- Linux with a StatusNotifierItem tray host. The app is developed and tested
  on KDE Plasma 6 with CachyOS; other compliant tray hosts may work but are not
  tested.
- A Sennheiser/EPOS GSP 370 headset and its USB dongle (`1395:009a`). Other
  headsets are not supported.
- Read access to the dongle's serial port. The packaged udev rule grants this
  to the active seat; systems without logind can use the `uucp` group instead.
- `notify-send` (usually from `libnotify`) for low-battery notifications.
- KDE Plasma 6 and `plasma-workspace` only if you want the optional panel
  widget.

## Install

### Arch Linux / CachyOS (AUR)

```bash
yay -S gsp370-tray
# or
paru -S gsp370-tray
```

The package installs the tray app, udev rule, desktop entry and Plasma widget.
Replug the dongle once if the tray reports a permission error after install.

See [Installation](docs/installation.md) for package contents, updates and
uninstallation.

### Prebuilt release

Download `gsp370-tray-<version>-x86_64.tar.gz` from
[GitHub Releases](https://github.com/NasirA1991/gsp370-tray/releases), unpack
it, and follow the included `INSTALL.txt`. The archive contains the binary,
desktop file, icon, udev rule, Plasma widget, documentation and licences.

### Build from source

Requirements: Rust 1.89 or newer and a C linker. `make` and Python 3 are only
needed for the optional helper targets and pseudo-terminal tests.

```bash
git clone https://github.com/NasirA1991/gsp370-tray.git
cd gsp370-tray
cargo build --release --locked --manifest-path rust/Cargo.toml
./scripts/install.sh
```

`scripts/install.sh` builds as your user, then asks for sudo to install under
`/usr/local`. It also installs the desktop files, Plasma widget and udev rule.
Use `PREFIX=/usr ./scripts/install.sh` to install somewhere else, or
`sudo ./scripts/install.sh --system` when you have already built as a user.

For the full source-build and manual-install instructions, see
[Installation](docs/installation.md).

## Usage

Start the tray:

```bash
gsp370-tray
```

Click the tray icon for the menu. It shows the current state, sidetone options
in 10 % steps, refresh, low-battery alerts, start-at-login and quit. Settings
are stored in `$XDG_CONFIG_HOME/gsp370-tray/config` (normally
`~/.config/gsp370-tray/config`).

Command-line examples:

```bash
gsp370-tray --battery          # print the percentage
gsp370-tray --probe            # show device, link state and battery
gsp370-tray --status           # cached status, with a probe fallback
gsp370-tray --set-sidetone 30  # 0 = off, 100 = maximum
gsp370-tray --device /dev/serial/by-id/usb-... --battery
```

For scripts, prefer `--status`: it reads the tray's short-lived status cache
and only opens the serial port when the cache is missing or stale. See the
[CLI and configuration reference](docs/usage.md#command-line-interface) for
output fields and exit codes.

Sidetone has no hardware read-back. The menu shows the last value set from the
menu; resetting the headset or changing sidetone from another host can make
that value stale. Sidetone is audible only while the microphone is in use.

### KDE Plasma widget

The package includes an optional Plasma 6 widget. Right-click the panel, choose
**Enter Edit Mode** → **Add Widgets**, and add **GSP 370 Battery**. It refreshes
once a minute, hides while the headset is off, and shows `!` when it cannot
reach the dongle.

See [Usage](docs/usage.md#kde-plasma-widget) for details.

## Limitations

- Only the GSP 370 with USB ID `1395:009a` is supported. Other Sennheiser/EPOS
  models have not been tested and are not claimed to work.
- The tray uses StatusNotifierItem. Desktops without an SNI host will not show
  the icon.
- Hiding the icon while the headset is off depends on KDE's **Only show when
  relevant** system-tray setting. Other tray hosts may keep the icon visible.
- Sidetone cannot be read back from the headset. The tray stores only the value
  saved from its menu and cannot verify the headset's actual level.
- The app does not expose a charging flag, firmware version, equalizer,
  surround processing, microphone processing or firmware updates.
- The prebuilt release archive is x86_64 only. The AUR package declares both
  x86_64 and aarch64, but only x86_64 release artifacts are built; source builds
  are not tied to x86_64.

## Troubleshooting

### Permission denied on `/dev/ttyACM*`

Install the udev rule and replug the dongle:

```bash
# AUR install:
sudo install -Dm644 \
  /usr/lib/udev/rules.d/70-gsp370.rules \
  /etc/udev/rules.d/70-gsp370.rules
# Source checkout:
sudo install -Dm644 \
  packaging/70-gsp370.rules \
  /etc/udev/rules.d/70-gsp370.rules

sudo udevadm control --reload-rules
sudo udevadm trigger --action=add /sys/class/tty/ttyACM0
```

If your system does not apply logind seat ACLs, add your user to `uucp` and log
out/in. See [Troubleshooting](docs/troubleshooting.md#permission-denied) for
the full procedure.

### Dongle not found or headset off

```bash
lsusb -d 1395:009a
ls -l /dev/ttyACM*
gsp370-tray --probe
```

The headset must be powered on and in range. A dongle that reports the headset
as off is working correctly; it simply cannot reach the headset.

### No tray icon

Confirm that your desktop's system tray is running and that the app is not in
the tray's overflow menu. The app uses StatusNotifierItem, so a desktop that
does not implement it cannot display the icon.

More fixes are in [Troubleshooting](docs/troubleshooting.md).

## Documentation

| Document | Contents |
|---|---|
| [Installation](docs/installation.md) | AUR, release archive, source build, manual paths, permissions, uninstall |
| [Usage and CLI](docs/usage.md) | Tray menu, CLI reference, configuration, environment variables, Plasma widget |
| [Troubleshooting](docs/troubleshooting.md) | Permission, dongle, tray, widget, sidetone and notification problems |
| [Architecture](docs/architecture.md) | Process model, modules, status cache and test strategy |
| [Protocol notes](docs/protocol.md) | Reverse-engineered serial framing, commands and evidence |
| [Contributing](CONTRIBUTING.md) | Development setup, tests, pull requests and releases |
| [Security](SECURITY.md) | Supported versions and private vulnerability reporting |
| [Changelog](CHANGELOG.md) | Release history |

## Contributing and support

Bug reports and feature requests are welcome in the
[issue tracker](https://github.com/NasirA1991/gsp370-tray/issues). Include the
output of `gsp370-tray --probe` and your installation method when reporting a
device problem. Before opening a pull request, see
[Contributing](CONTRIBUTING.md).

## License

The application is licensed under the MIT License. The embedded Noto Sans
Medium digit subset is licensed under the SIL Open Font License 1.1; see
[`rust/assets/NotoSans-OFL.txt`](https://github.com/NasirA1991/gsp370-tray/blob/main/rust/assets/NotoSans-OFL.txt).
Packaged installs place it under `/usr/share/licenses/gsp370-tray/`; source
installs use `<prefix>/share/licenses/gsp370-tray/`.

GSP 370 Tray is not affiliated with or endorsed by EPOS, Sennheiser or Demant.
