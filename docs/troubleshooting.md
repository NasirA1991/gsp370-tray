# Troubleshooting

Start with the diagnostic commands below; they distinguish a missing dongle,
a headset that is switched off, and a permission problem without guessing.

```bash
gsp370-tray --version
gsp370-tray --status
gsp370-tray --probe
```

`--status` uses the tray cache when it is fresh and probes otherwise.
`--probe` always talks to the device and prints the raw diagnostic result.

## No tray icon

The app needs a StatusNotifierItem host. KDE Plasma provides one natively. On
GNOME, install and enable an AppIndicator/StatusNotifier extension; a bare
GNOME session has no tray area for this application.

Check that the process is running:

```bash
pgrep -a gsp370-tray
```

The app is single-instance. If a copy is already running, starting another one
exits immediately. Run it from a terminal to see startup errors:

```bash
gsp370-tray
```

On KDE, the icon can be hidden for two expected reasons:

- the panel is in **only show when relevant** mode and the headset is off;
- the icon is in the panel's overflow/chevron area.

`gsp370-tray --status` should report `state=off` in the first case. The app is
also categorised as an application status item, not as a hardware device, so
look for it with the other status icons rather than in a separate hardware
section.

## The headset is not detected

First establish what the app sees:

| Output | Meaning |
|---|---|
| `state=no-dongle` | The USB dongle is not present or not enumerating as the expected device. |
| `state=off` | The dongle is present, but the headset is switched off or out of range. |
| `state=permission` | The dongle is present, but the serial node cannot be opened. |
| `state=error` | The dongle was found, but the serial exchange failed. |

Check USB enumeration and the tty node:

```bash
lsusb -d 1395:009a
ls -l /dev/ttyACM*
```

The supported dongle is USB `1395:009a`. Other GSP models and other USB IDs are
not supported. If `lsusb` finds nothing, try another USB port, replug the
dongle, and check the kernel log:

```bash
sudo dmesg | tail -n 40
```

A working dongle normally creates a `cdc_acm` tty such as `/dev/ttyACM0`. If it
appears as a different node, or if another ACM device occupies the first node,
select it explicitly:

```bash
gsp370-tray --device /dev/ttyACM1 --probe
```

The `/dev/serial/by-id/` symlinks are also accepted. The app validates that the
resolved path is under `/dev`; paths outside it are rejected.

If the dongle is present but the headset is off, switch the headset on and wait
a few seconds. The dongle keeps the USB device enumerated while the headset is
off, so `no-dongle` and `off` are different problems.

## Permission denied

`--status` reports `state=permission` and `--probe` reports a permission-style
error when the dongle is visible but the serial node is not accessible.

Check the rule and the node:

```bash
ls -l /dev/ttyACM0
getfacl /dev/ttyACM0 2>/dev/null
udevadm info --query=all --name=/dev/ttyACM0 | grep -E 'ID_VENDOR_ID|ID_MODEL_ID|TAG|GROUP|MODE'
```

The expected rule tags the device with `uaccess`, sets group `uucp`, mode
`0660`, and tells ModemManager to ignore it. AUR installs the rule in
`/usr/lib/udev/rules.d`; `scripts/install.sh` installs it in
`/etc/udev/rules.d`. Reload and re-apply it after installation or repair:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add /sys/class/tty/ttyACM0
```

Then unplug and replug the dongle. The trigger path must match the actual tty
node (`ttyACM1`, and so on).

If your system does not apply logind seat ACLs, use the group fallback and log
out and back in:

```bash
sudo usermod -aG uucp "$USER"
id -nG
```

`id -nG` must list `uucp` in the new session. Running the app with `sudo` is not
a supported workaround; it can create root-owned settings and does not fix the
desktop session's access.

If you installed both the AUR package and a source build, remove the unused
copy so there is only one binary and one rule. See
[Installation](installation.md#uninstall).

## Battery or status looks stale

The tray refreshes once per minute, and the shared status cache is valid for
90 seconds. A widget or CLI call made immediately after a change can therefore
show the previous value. Force a probe with:

```bash
gsp370-tray --status --device /dev/ttyACM0
```

`--status --device` bypasses the cache. The **Refresh now** tray menu item and
clicking the Plasma widget also request a fresh value.

The percentage comes from the headset's own gauge. The app does not calibrate
it, and it may remain at 100 % for a long time or move in steps. If the headset
is off, the state is `off`, not a battery reading.

## Sidetone does not work

Sidetone is audible only while the microphone stream is open. A silent
microphone, a muted input or an application that has not opened the microphone
will make the setting appear to do nothing.

The device has no sidetone read-back. The menu shows the last value saved from
the menu, not a value queried from the headset. This means the displayed value
can be stale after:

- resetting the headset;
- changing sidetone from another host;
- using `--set-sidetone`, which changes the headset but does not update the
  tray's saved config.

Use the tray menu when you want the value to persist in the menu. Resetting the
headset or reconnecting it can also reset the hardware level; set it again from
the menu if needed.

## No low-battery notifications

Notifications require `notify-send`:

```bash
command -v notify-send
notify-send "GSP 370" "Notification test"
```

If the test notification does not appear, check the desktop's notification
settings and Do Not Disturb mode before troubleshooting the app.

The tray sends notifications at 20 %, 10 % and 5 %. Each threshold fires once
until the battery recovers above 25 % or rises by more than two percentage
points. Alerts can be disabled from the tray menu; check that **Low battery
alerts** is enabled, or inspect the `alerts` key in the config file. See
[Usage](usage.md#low-battery-notifications).

Notifications are sent only while the tray is running.

## Plasma widget problems

The widget is Plasma 6-only. If **GSP 370 Battery** is not listed in **Add
Widgets**, verify that the package installed it:

```bash
kpackagetool6 --type Plasma/Applet --list | grep -i gsp370
```

The AUR package installs the applet under
`/usr/share/plasma/plasmoids/`; a source install uses
`/usr/local/share/plasma/plasmoids/`. Log out and back in if Plasma does not
pick up a newly installed applet.

A `!` in the widget means `--status` returned a problem or no usable state.
Run it in a terminal for the underlying output:

```bash
gsp370-tray --status
```

The widget invokes the binary from `~/.local/bin`, `/usr/bin` or
`/usr/local/bin`, in that order. A binary installed under a different prefix
will not be found; install it in one of those locations or create a symlink
there.

The widget hides while the headset is off, matching the tray's **only show when
relevant** behaviour. This is not a crash. Click the widget to refresh it
immediately.

## Start at login does not work

The tray's toggle writes:

```text
~/.config/autostart/gsp370-tray.desktop
```

Check that the file exists and that its `Exec=` path is still valid. The path
is captured when the toggle is enabled, so moving or reinstalling the binary
requires toggling the setting off and on again.

The desktop session must support XDG autostart. If the process starts but no
icon is visible, check the **No tray icon** section above; the headset may
simply be off.

## Known limitations

- Only the GSP 370 dongle with USB ID `1395:009a` is supported.
- The headset connects through its USB dongle, not Bluetooth.
- The app controls battery reporting and sidetone. It does not provide audio
  routing, an equalizer or firmware updates.
- The tray requires a StatusNotifierItem host; the widget requires KDE Plasma 6.
- Linux is the only supported platform.

These limitations are part of the project's scope, not installation bugs. See
the [README](../README.md#limitations) for the user-facing summary.

## Collecting information for a bug report

Include the following when opening an issue:

```bash
gsp370-tray --version
gsp370-tray --status
gsp370-tray --probe
lsusb -d 1395:009a
ls -l /dev/ttyACM*
udevadm info --query=all --name=/dev/ttyACM0 | grep -E 'ID_VENDOR_ID|ID_MODEL_ID|TAG|GROUP|MODE'
```

Also mention:

- distribution and version;
- desktop environment and Plasma version, if relevant;
- whether you installed from the AUR, the prebuilt archive or source;
- whether `--probe` succeeds as root (this helps separate permissions from
  protocol problems);
- any terminal output from the tray.

The app does not write a log file. Run it from a terminal to capture startup
errors. Use the [issue tracker](https://github.com/NasirA1991/gsp370-tray/issues)
for bug reports and feature requests.
