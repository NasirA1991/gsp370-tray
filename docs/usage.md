# Usage, CLI and configuration

This page covers the day-to-day behaviour of GSP 370 Tray. For installation,
see [Installation](installation.md); for the serial protocol, see
[Protocol notes](protocol.md).

## Running the tray

```bash
gsp370-tray
```

The tray uses StatusNotifierItem, so it needs a compatible system-tray host.
The app is single-instance: starting it again exits without creating a second
tray icon. On KDE Plasma 6, the icon is placed with normal application status
items rather than under “Hardware”.

Click the tray icon to open the menu. It contains:

| Item | Behaviour |
|---|---|
| Status line | Current headset state and battery percentage. |
| **Sidetone** | `Off` or 10–100 % in 10 % steps. Enabled only while the headset is connected. |
| **Refresh now** | Forces an immediate status probe. |
| **Low battery alerts** | Enables or disables the 20 %, 10 % and 5 % notifications. |
| **Start at login** | Creates or removes an XDG autostart entry. |
| **Quit** | Stops the tray. |

The tooltip shows the same status, the last sidetone value saved from the menu,
and the serial device path when connected.

Settings are stored in `$XDG_CONFIG_HOME/gsp370-tray/config`, or
`~/.config/gsp370-tray/config` when `XDG_CONFIG_HOME` is unset.

## Low-battery notifications

When alerts are enabled, the tray sends a desktop notification the first time
the battery crosses 20 %, 10 % and 5 %. It does not repeat the same threshold
until the battery has recovered above 25 % or risen by more than two percentage
points. Notifications use `notify-send` from `libnotify`; if that command is
missing, the tray continues to work without notifications.

Desktop notification settings, such as Do Not Disturb, still apply.

## Command-line interface

The binary has both a tray mode and one-shot CLI actions. With no arguments it
starts the tray. `--device` may be used alone to start the tray with an explicit
serial path.

```text
Usage: gsp370-tray [options]

Options:
  -b, --battery              print battery percentage and exit
  -p, --probe                print device, link and battery status
      --status               machine-readable cached/probed status
  -s, --set-sidetone <0-100> set sidetone and exit (0 = off)
  -d, --device <path>        override serial device (CLI or tray)
  -h, --help                 show this help
  -v, --version              show version
```

Both `--device <path>` and `--device=<path>` are accepted.
`--set-sidetone=<value>` is accepted as well.

### `--battery`

Prints only the battery percentage as an integer, or `off` when the dongle
reports that the headset is switched off or out of range.

```console
$ gsp370-tray --battery
87
```

| Result | Stdout | Exit code |
|---|---|---:|
| Connected | percentage, for example `87` | 0 |
| Headset off/out of range | `off` | 2 |
| Device or permission error | nothing | 1 |

Errors are written to stderr.

### `--probe`

Prints a diagnostic key-value report: the serial device, link status and, when
available, the battery.

```console
$ gsp370-tray --probe
device=/dev/ttyACM0
status=on
battery=87
```

A headset that is off produces `status=headset-off` and exit code 2. A dongle
or permission failure produces a `status=` diagnostic value for the underlying
error and exit code 1. `--probe` is intended for diagnosis and bug reports; use
`--status` for scripts.

### `--status`

Returns the tray's cached status when the cache is fresh, and otherwise probes
the dongle once. It is the interface used by the Plasma widget.

```console
$ gsp370-tray --status
state=on
battery=87
device=/dev/ttyACM0
```

The fields are:

| Field | Meaning |
|---|---|
| `state` | `on`, `off`, `no-dongle`, `permission`, `error` or `unknown` |
| `battery` | Percentage, present only when the state is `on` |
| `device` | Serial device path, present when it is known |

The exit codes are stable for scripting:

| State | Exit code |
|---|---:|
| `on` or `unknown` | 0 |
| `off` | 2 |
| `no-dongle`, `permission`, `error` | 1 |

A fresh cache is reused for up to 90 seconds. After that, `--status` performs
one probe. `--status --device` always uses the supplied device and bypasses the
cache.

### `--set-sidetone`

Sets sidetone to an integer from 0 to 100, where 0 is off and 100 is the
maximum. The device exposes eleven audible levels; the app maps the requested
percentage onto those levels. The tray menu uses 10 % steps, while this option
accepts any integer in the range.

```console
$ gsp370-tray --set-sidetone 30
sidetone=30%
```

A successful command exits 0. A headset that is off exits 2; any other error
exits 1.

The CLI does **not** update the tray's saved sidetone value. Use the tray menu
if you want the selected value to persist and appear in the menu. The headset
does not report its current sidetone level, so the menu can only show the last
value saved from the menu.

Sidetone is audible only while the microphone stream is open. Resetting the
headset or setting sidetone from another host can make the displayed value
stale.

### `--device`

Overrides automatic dongle detection for any action, or for tray mode when used
without another action:

```bash
gsp370-tray --device /dev/ttyACM1
gsp370-tray --device /dev/ttyACM1 --battery
gsp370-tray --device=/dev/serial/by-id/usb-... --probe
```

The path is resolved with `realpath`-style canonicalisation and must resolve
under `/dev`; symlinks such as `/dev/serial/by-id/...` are accepted as long as
their target remains under `/dev`. This prevents pointing the app at arbitrary
files outside the device tree. An explicit `--device` takes precedence over
`GSP370_TTY`; if neither is set, the app scans `/sys/class/tty/ttyACM*` for USB
vendor/product `1395:009a`.

## Configuration

The config file is:

```text
$XDG_CONFIG_HOME/gsp370-tray/config
```

or `~/.config/gsp370-tray/config` if `XDG_CONFIG_HOME` is unset. It is a small
`key=value` file written by the app:

```ini
# gsp370-tray config v1
sidetone=50
alerts=1
```

| Key | Default | Meaning |
|---|---:|---|
| `sidetone` | `50` | Last sidetone percentage saved by the tray menu. Values 101–255 are clamped to 100; other out-of-range values are ignored. |
| `alerts` | `1` | `0`, `false` or `no` disables low-battery notifications; any other value enables them. |

Unknown keys and lines without `=` are ignored. Invalid numeric values are
ignored. The file is written atomically when a setting changes. You can edit it
while the tray is stopped; there is no live reload.

The tray's start-at-login toggle creates or removes:

```text
$XDG_CONFIG_HOME/autostart/gsp370-tray.desktop
```

That desktop entry runs the executable path that was current when the toggle
was enabled. If you move or reinstall the binary, toggle the setting off and on
again to update the path.

## Environment variables

| Variable | Purpose |
|---|---|
| `GSP370_TTY` | Default serial device when `--device` is not supplied. Ignored when `--device` is present. |
| `GSP370_TEXT_COLOR` | Icon text colour as `R,G,B` bytes, for example `245,247,250`. Useful for unusual panel themes. |
| `GSP370_STATE_DIR` | Overrides the runtime state directory. Primarily intended for tests; not needed in normal use. |

`GSP370_TTY` is useful for tests and unusual device names. The path goes through
the same `/dev` validation as `--device`.

If `GSP370_TEXT_COLOR` is unset, the icon queries the Freedesktop
`org.freedesktop.appearance.color-scheme` portal setting, then falls back to
KDE's window background and `GTK_THEME`. The tray pixmap is not recoloured by
the compositor, so the app chooses a light or dark text colour itself.

## Status cache

While running, the tray writes a small cache to:

```text
$XDG_RUNTIME_DIR/gsp370-tray/status
```

The fallback is `~/.cache/gsp370-tray/status`, or
`$GSP370_STATE_DIR/status` when the override is set. The tray and the Plasma
widget use this file to avoid opening the serial port concurrently. Entries
older than 90 seconds are ignored, so the widget can probe directly after the
tray exits.

The cache is an implementation detail. Use `gsp370-tray --status` rather than
reading it directly. Do not edit the file while the tray is running.

## KDE Plasma widget

The package includes the optional **GSP 370 Battery** widget for KDE Plasma 6.
It displays the battery as normal panel text, for example `84%`, instead of a
22 px tray icon.

To add it:

1. Right-click the panel and choose **Enter Edit Mode**.
2. Choose **Add Widgets**.
3. Search for **GSP 370 Battery** and add it.

The widget:

- refreshes every 60 seconds;
- hides while the headset is off;
- shows `!` for a missing dongle, permission failure or connection error;
- refreshes immediately when clicked;
- follows the KDE panel's light/dark background.

It invokes `gsp370-tray --status`, looking for the binary in
`~/.local/bin`, `/usr/bin` and `/usr/local/bin`, in that order. If the widget
shows `!`, run `gsp370-tray --status` in a terminal to see the underlying state.
The widget requires `plasma-workspace`; it is not available on non-Plasma
desktops.

## Desktop integration

The desktop entry starts `gsp370-tray` without a terminal. The icon is
`gsp370-tray.svg`. The app does not create a log file; run it from a terminal
to see error messages:

```bash
gsp370-tray --device /dev/ttyACM0
```

For uninstall and permission setup, see [Installation](installation.md).
