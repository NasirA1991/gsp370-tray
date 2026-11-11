# Architecture

GSP 370 Tray is a small Rust application with one optional companion: a KDE
Plasma widget that reads the same status through the CLI. There is no daemon,
no network service and no public library API.

## Project layout

| Path | Responsibility |
|---|---|
| `rust/src/main.rs` | CLI parsing, tray menu, configuration, autostart, notifications, serial worker and status cache updates |
| `rust/src/serial.rs` | USB tty discovery, `/dev` path validation, termios setup and device commands |
| `rust/src/protocol.rs` | Frame constants, command construction, reply parser and sidetone mapping |
| `rust/src/status.rs` | Runtime state directory, status cache, atomic writes and single-instance lock path |
| `rust/src/icon.rs` | Tray pixmaps, theme-aware text colour and small-size bitmap faces |
| `tests/e2e_pty.py` | Pseudo-terminal integration test with a fake dongle |
| `packaging/` | udev rule, desktop entry, icon, AUR template and Plasma widget |
| `.github/workflows/` | CI and tagged-release automation |

## Runtime model

With no CLI action, the process has three cooperating parts:

1. **StatusNotifierItem service.** `ksni` publishes the tray icon and menu over
   D-Bus. The main thread parks after starting the service.
2. **Serial worker.** A dedicated thread owns all access to the dongle. It
   polls the device, applies menu commands and writes the shared status cache.
3. **Status cache.** A small file under the per-user runtime directory lets
   the Plasma widget and `--status` read the last result without opening the
   serial port concurrently.

```text
 tray menu ──Command──┐
                      ▼
                serial worker ──probe──► dongle (/dev/ttyACM*)
                      │
                      ├──Handle::update──► StatusNotifierItem
                      └──status::write───► status cache
                                               ▲
 Plasma widget ──gsp370-tray --status──────────┘
```

In tray mode, the tray's own polling and menu commands go through the serial
worker. The widget never opens `/dev/ttyACM*` directly; it invokes the CLI,
which either reads a fresh cache entry or performs one probe. One-shot CLI
commands can still open the device while the tray is running and may briefly
contend with the worker.

### Single instance

Tray mode takes an exclusive `flock` on `tray.lock` inside the state directory.
A second tray process exits successfully without creating another icon. CLI
actions (`--battery`, `--probe`, `--status`, `--set-sidetone`) do not take the
tray lock. They can run while the tray is active, but direct device commands
may briefly contend with its polling.

### Commands and polling

The tray menu communicates with the serial worker through a
`std::sync::mpsc` channel:

- `Refresh` wakes the worker immediately;
- `SetSidetone(percent)` opens the device, applies the level and saves the
  new value only after success;
- `SetAlerts(enabled)` updates the config and the menu.

The worker sleeps until the next poll or a command. Poll intervals depend on
the last state:

| State | Interval |
|---|---:|
| connected | 60 s |
| headset off | 20 s |
| dongle missing | 15 s |
| permission denied | 60 s |
| serial/protocol error | 30 s |
| starting/unknown | 5 s |

After a successful probe, the worker updates the tray through
`Handle::update`, writes the state cache, and evaluates low-battery
notifications. The UI update and the cache are intentionally separate: a
widget can keep showing the last cached value while the tray is alive, and can
probe directly after the tray exits.

## Serial layer

`Device::open` chooses the device in this order:

1. an explicit `--device` value;
2. `GSP370_TTY`;
3. the first `/dev/ttyACM*` whose sysfs USB IDs are `1395:009a`.

Explicit paths and the environment variable are canonicalised and must resolve
under `/dev`; `/dev/serial/by-id/...` symlinks are accepted. Autodetection
reads `/sys/class/tty/ttyACM*/device/../idVendor` and `idProduct`.

The port is configured with raw termios at 921600 baud, 8 data bits, no parity,
one stop bit and no flow control. Opening waits 150 ms for the dongle to
settle. Commands use a 100 ms pacing delay and a 1.5 s reply timeout, matching
the behaviour observed in the reference implementations. See
[Protocol notes](protocol.md) for frame layout, opcodes and the evidence behind
them.

The serial worker converts command failures into `Probe` outcomes:

| Outcome | Cache/status state |
|---|---|
| battery read succeeds | `on` |
| status `0x0C` | `off` |
| dongle not found | `no-dongle` |
| `EACCES`/`EPERM` while opening | `permission` |
| other serial/protocol failure | `error` |

The CLI keeps the same mapping so `--battery`, `--probe`, `--status` and the
tray describe the device consistently.

## Status cache

`rust/src/status.rs` chooses the state directory in this order:

1. `GSP370_STATE_DIR`;
2. `$XDG_RUNTIME_DIR/gsp370-tray`;
3. `$HOME/.cache/gsp370-tray`;
4. a temporary directory named after the effective UID.

The directory is created user-private (mode `0700`). `status` contains
`version=1`, `state`, optional `battery` and optional `device`; entries older
than 90 seconds are ignored. Writes go through a private temporary file and
`rename`, so readers never see a truncated file. Config and autostart files use
the same atomic-write helper.

The tray writes the cache after each probe. `--status` reads a fresh entry and
otherwise probes; `--status --device` always bypasses the cache because an
explicit device may differ from the cached one.

## Icons and theme

`icon.rs` generates several pixmap sizes (`16`, `22`, `24`, `32`, `48`, `64`,
`128`) for every state. Small sizes use bitmap faces that stay legible in a
22 px panel slot; larger sizes use the embedded Noto Sans Medium subset. The
icon is transparent and the text colour is selected from:

1. `GSP370_TEXT_COLOR` (`R,G,B`), if set;
2. the Freedesktop `org.freedesktop.appearance.color-scheme` portal setting;
3. KDE's `[Colors:Window]` background brightness;
4. the `GTK_THEME` name.

Disconnected, permission and error states use distinct accents and a compact
glyph; the headset-off state uses a passive StatusNotifierItem status so tray
hosts that hide irrelevant items can hide it. The Plasma widget provides a
text alternative for users who prefer a larger, native panel label.

## Configuration and desktop integration

The config is a two-key `key=value` file:

```text
~/.config/gsp370-tray/config
```

`sidetone` is the last value saved by the tray menu; `alerts` controls
low-battery notifications. The parser is deliberately small and the file is
rewritten atomically. The start-at-login menu item writes or removes an XDG
autostart entry whose `Exec=` is the current executable path. Neither file is
owned by the package, so uninstalling leaves user settings in place.

Low-battery notifications invoke `notify-send`; failure to start it is
non-fatal. Notifications are evaluated in the serial worker only when the
battery is readable, and each threshold is latched until the battery recovers.

## Packaging and release flow

The project has three installation paths:

- **AUR**: `packaging/aur/PKGBUILD.in` is rendered by the release workflow with
  the new version and source checksum, then committed to the AUR repository.
- **Prebuilt archive**: the release workflow builds a binary bundle containing
  `INSTALL.txt`, the desktop files, udev rule, widget, docs and licences.
- **Source**: `scripts/install.sh` builds as the user and re-runs itself under
  sudo to install into `/usr/local`.

`packaging/70-gsp370.rules` grants the active seat access (`uaccess`) and sets
the `uucp` group fallback. The AUR install hook reloads udev. The widget is
installed under `share/plasma/plasmoids/` and is optional.

The release workflow is tag-driven:

```text
v* tag -> test/build -> source + binary archives -> Arch package
       -> GitHub release -> AUR commit
```

CI runs formatting, clippy, unit tests, the PTY suite, the MSRV check, an
aarch64 compile check and `cargo audit`. See
[`.github/workflows/ci.yml`](https://github.com/NasirA1991/gsp370-tray/blob/main/.github/workflows/ci.yml)
for the exact matrix.

## Tests

- Unit tests live beside the modules in `rust/src/*.rs`: protocol framing and
  resynchronisation, serial helpers, status parsing, config parsing, sidetone
  mapping, notification thresholds and icon invariants.
- `tests/e2e_pty.py` creates a pseudo-terminal, emulates the dongle and
  exercises termios setup, command/reply framing, asynchronous event skipping,
  timeout/error statuses and the sidetone sequence.
- The e2e suite does not require the real headset. Hardware-only behaviour,
  such as the actual battery gauge or audible sidetone, is noted in
  [Protocol notes](protocol.md#evidence) rather than asserted by tests.

## Design boundaries

- One process and one serial worker keep device contention and state
  synchronisation simple.
- The status cache is a compatibility boundary between the tray and the
  widget, not a general-purpose API.
- The app does not implement audio routing, equalizer processing or firmware
  flashing; those are outside the observed control protocol and the project
  scope.
- Linux and a StatusNotifierItem host are required. The widget additionally
  requires KDE Plasma 6.
