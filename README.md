# GSP 370 Tray

[![CI](https://github.com/NasirA1991/gsp370-tray/actions/workflows/ci.yml/badge.svg)](https://github.com/NasirA1991/gsp370-tray/actions/workflows/ci.yml)
[![AUR](https://img.shields.io/aur/version/gsp370-tray?label=AUR)](https://aur.archlinux.org/packages/gsp370-tray)

A deliberately small Linux tray app for the **Sennheiser/EPOS GSP 370**
wireless headset (USB `1395:009a`).  It replaces the hardware-facing parts of
the discontinued EPOS Gaming Suite without Qt, GTK, Electron, a web view, or
audio processing.

- **Battery percentage in the tray**, drawn as a ring with the number inside
- **Low-battery notifications** at 20 %, 10 % and 5 %
- **Sidetone control** from a tray submenu
- **Headset on/off** detection
- Categorized as an ordinary application (not “Hardware”) so KDE places it with your apps
- **Start at login** toggle and automatic reconnect after unplug/replug
- **CLI** for scripting and testing

The whole tray is a Rust binary using `ksni`/`zbus` directly.  There is no
toolkit event loop: a serial worker thread polls the dongle once a minute and
updates the StatusNotifierItem over D-Bus.

![Tray icon example](docs/tray-icon.png)

## Performance

Measured on this machine (KDE Plasma Wayland), 20-second idle window:

| Build | RSS | PSS | Private anonymous |
|---|---:|---:|---:|
| **GSP 370 Tray 2.0 (Rust + raw SNI)** | **11.3 MiB** | **8.8 MiB** | **7.3 MiB** |
| Previous Qt Widgets version | 68.4 MiB | 24.8 MiB | 12.8 MiB |
| Minimal Qt Widgets tray (baseline) | 52.5 MiB | 14.4 MiB | 9.3 MiB |
| Minimal GTK3 + AppIndicator tray (baseline) | 29.8 MiB | 11.8 MiB | 6.4 MiB |

RSS counts shared library pages that are also mapped by other processes; PSS
is the fairer “cost of this app” number.  The Rust build is roughly **3×
lighter in PSS** and **~7× lighter in RSS** than the Qt version.  Idle CPU was
**≈0.1 %** over 20 s (one battery poll), and the binary is ~1.7 MiB.

Qt is not automatically lighter than GTK, or vice versa: both are toolkit
stacks.  For a tray-only app the meaningful win is not choosing a different
toolkit but not using one at all.

## What the Windows suite offers, and what this app does

The Gaming Suite is a general EPOS app.  For the GSP 370 its hardware-facing
features are battery, sidetone and firmware updates.  The EQ, surround,
reverb, noise gate and microphone processing are Windows audio-processing
objects (software effects), not settings stored in this headset.

| Feature | EPOS Gaming Suite (GSP 370) | GSP 370 Tray |
|---|---|---|
| Battery percentage | Yes | Yes, in the tray icon and tooltip |
| Low-battery alert | Yes | Yes, 20 / 10 / 5 % |
| Sidetone | Yes | Yes, 0–100 % menu, mapped to levels 6–16 |
| Headset off/out of range | Yes | Yes |
| Headset/dongle firmware update | Yes, two components | **Not implemented** (deliberately) |
| EQ / surround / reverb / noise gate | Software APO effects | Not included |
| Tray icon | Yes | Yes, percentage ring |
| Autostart | Yes | Yes, toggle in the menu |

Firmware flashing is not implemented.  A wrong image or interrupted write can
brick the dongle or headset, and the update protocol was not validated for that
purpose.  Use the original Windows suite in a VM if a firmware update is
required.  The equalizer is host-side DSP on Windows and is intentionally left
out of this low-memory tray; see [docs/protocol.md](docs/protocol.md) for the
factory preset values if you want to reproduce them in EasyEffects.

## Install

### Arch Linux / CachyOS (AUR)

```bash
yay -S gsp370-tray
# or
paru -S gsp370-tray
```

The package installs the udev rule and reloads it.  If the dongle was already
plugged in and the tray reports a permission error, unplug/replug it once.

### Release binary

Download `gsp370-tray-2.0.0-x86_64.tar.gz` from
[GitHub Releases](https://github.com/NasirA1991/gsp370-tray/releases), extract
it, and install the binary/desktop files manually, or use the source build
below.

### From source (Arch / CachyOS)

Hardware access is the same udev rule as before; the Rust build has no extra
runtime dependencies beyond the system D-Bus and libc.

```bash
sudo pacman -S --needed rust
git clone https://github.com/NasirA1991/gsp370-tray.git
cd gsp370-tray

cargo build --release --locked --manifest-path rust/Cargo.toml
cargo test  --release --locked --manifest-path rust/Cargo.toml
python3 tests/e2e_pty.py ./rust/target/release/gsp370-tray

sudo ./scripts/install.sh
```

Other distributions need a Rust toolchain (Rust 1.80 or newer) and a C linker:

```bash
# Debian / Ubuntu
sudo apt install build-essential rustc cargo

# Fedora
sudo dnf install gcc cargo rust
```

`scripts/install.sh` uses the release binary if it exists, otherwise runs
`cargo build`.  It installs the binary under `/usr/local`, installs the desktop
file and icon, and copies `packaging/70-gsp370.rules` to
`/etc/udev/rules.d/70-gsp370.rules`.  It reloads udev and re-adds the GSP 370
tty node, which makes the seat ACL apply to an already-connected dongle.

The udev rule uses `TAG+="uaccess"` so the logged-in seat user can open the
CDC-ACM port without joining a group.  `GROUP="uucp"` is a fallback for
systems without logind seat ACLs; in that case add yourself to `uucp` and log
out/in.

Start **GSP 370 Tray** from the application menu, or run:

```bash
gsp370-tray
```

Click the tray icon for the menu.  **Start at login** creates
`~/.config/autostart/gsp370-tray.desktop`.

## CLI

```bash
gsp370-tray --battery          # prints e.g. 87; prints "off" and exits 2 if headset is off
gsp370-tray --probe            # device path + status + battery
gsp370-tray --set-sidetone 30  # 0 = off, 100 = max
gsp370-tray --device /dev/ttyACM1 --battery   # override auto-detection
```

Exit codes: `0` success, `1` no dongle / permission / protocol error,
`2` headset is off.

Settings are stored in `~/.config/gsp370-tray/config` (two lines:
`sidetone=` and `alerts=`).  The app has no GUI window and no daemon beyond the
tray process.

## How it works

The GSP 370 dongle exposes six USB interfaces.  The HID interface (3) only
sends media keys.  Settings go over the CDC-ACM serial interface (4), which
Linux exposes as `/dev/ttyACM0` at **921600 8N1**:

```
command:  01 <opcode> fc <len> <params...>
reply:    04 0e <len> 01 <opcode> fc <status> <values...>
```

- `0x9C` reads battery percent.
- `0xA0` / `0xA1` prepare sidetone on/off.
- `0xA2 <level>` applies a sidetone level (`0`, or `6..16`).
- Status `0x0C` means the headset is off or out of range.

The full reverse-engineering notes, evidence and unknowns are in
[docs/protocol.md](docs/protocol.md).

### Why the sidetone menu is not a read-back

The GSP 370 has no known command that returns the current sidetone level.  The
menu shows the last value **set by this app** and applies it when selected.  It
does not claim to read the headset's current setting.

### Low overhead

- one Rust process, one serial worker thread
- raw StatusNotifierItem over D-Bus; no Qt/GTK toolkit
- serial port opened only for a poll/command, then closed
- 60 s battery poll while connected; 20 s while the headset is off; 15 s when
  the dongle is absent
- icon rendered only when state or battery changes
- no audio streams, no polling loop, no Electron/Node/Python runtime

## Hardware verification

The test unit is a GSP 370 dongle (`1395:009a`) on CachyOS, Linux 7.3.1,
KDE Plasma Wayland.  Verified on 2026-11-04:

```text
$ gsp370-tray --probe
device=/dev/ttyACM0
status=on
battery=84

$ gsp370-tray --set-sidetone 30
sidetone=30%
```

- The dongle auto-detected as `/dev/ttyACM0`; battery and sidetone commands
  completed with status 0.
- The Rust StatusNotifierItem registered in Plasma; the menu showed
  `Battery: 84%`, and the tooltip contained the battery, sidetone and port.
- **Sidetone → 30 %** from the tray menu updated the tooltip and persisted
  `sidetone=30` in `~/.config/gsp370-tray/config`.
- The **Start at login** toggle created and removed a valid autostart file.
- The previous Qt implementation was also hardware-tested (battery and
  sidetone), so the protocol is not merely a static reconstruction.
- Idle cost for the Rust build: **11.3 MiB RSS / 8.8 MiB PSS**, ≈0.1 % CPU
  over 20 s.
- `tests/e2e_pty.py` also verifies framing, headset-off status and the
  two-command sidetone sequence without the headset.

## Troubleshooting

### “Permission denied” / “Serial permission denied”

```bash
ls -l /dev/ttyACM0
getfacl /dev/ttyACM0
```

Install the rule and trigger it:

```bash
sudo install -Dm644 packaging/70-gsp370.rules /etc/udev/rules.d/70-gsp370.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add /sys/class/tty/ttyACM0
```

Then unplug/replug the dongle.  If `getfacl` still does not show your user,
add yourself to `uucp` and log out/in:

```bash
sudo usermod -aG uucp "$USER"
```

### “Dongle not found”

```bash
lsusb -d 1395:009a
ls -l /dev/ttyACM*
```

Try another USB port.  The dongle is full-speed USB and works through a hub,
but a flaky hub can cause reconnects.

### “No reply” / headset off

The headset must be powered on and in range.  `status 0x0C` is the dongle
saying it cannot reach the headset.  If the headset is on, make sure no other
program is holding the serial port:

```bash
lsof /dev/ttyACM0
```

### No tray icon

This app uses StatusNotifierItem.  On KDE Plasma, check that the System Tray
widget is present and that the app is not hidden in the tray's overflow menu.
Run `gsp370-tray --probe` from a terminal to check hardware access.

### Firmware update

Not supported.  Keep the official Windows installer/VM available if you need
to update the headset or dongle firmware.

## Development

```
rust/src/protocol.rs     framing, parsing, sidetone mapping
rust/src/serial.rs       libc termios serial transport and sysfs discovery
rust/src/icon.rs         tiny CPU-side ring/percentage icon renderer
rust/src/main.rs         SNI tray, menu, config, serial worker
tests/e2e_pty.py         pseudo-terminal end-to-end test (works with both builds)
packaging/               udev rule, .desktop, icon
```

Build and test:

```bash
cargo build --release --locked --manifest-path rust/Cargo.toml
cargo test  --release --locked --manifest-path rust/Cargo.toml
python3 tests/e2e_pty.py ./rust/target/release/gsp370-tray
```

## Credits and legal

This is an unofficial community project, not affiliated with or endorsed by
EPOS, Sennheiser or Demant.  “EPOS”, “Sennheiser” and “GSP” are trademarks of
their respective owners and are used only to identify the hardware.

Protocol facts were cross-checked against:

- [HeadsetControl GSP 370 branch](https://github.com/kaffeewolf/HeadsetControl/tree/add-gsp370-support)
  (implementation and captured command bytes)
- [eposctl](https://github.com/hiyufan/eposctl) (independent GSP 370
  implementation, Gaming Suite notes and factory EQ curves)
- The official EPOS Gaming Suite 1.12.2.1185 installer, used only for static
  interoperability analysis

The Rust app is MIT-licensed; see [LICENSE](LICENSE).  The icon digits use a
subset of DejaVu Sans Bold; the font license is in
[`rust/assets/DejaVu-LICENSE.txt`](rust/assets/DejaVu-LICENSE.txt).

Source: <https://github.com/NasirA1991/gsp370-tray>
