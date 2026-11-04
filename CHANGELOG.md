# Changelog

## 2.1.1 — 2026-11-04

- Fixed the Plasma widget's text colour: it now follows the KDE light/dark
  background instead of falling back to black.

## 2.1.0 — 2026-11-04

- Added an optional KDE Plasma panel widget that renders the battery as
  native panel text (`84%`) in the system font.
- The widget hides while the headset is off and refreshes every minute.
- The AUR package installs the widget alongside the tray app.

## 2.0.6 — 2026-11-04

- Report StatusNotifierItem `Passive` while the headset is off, so KDE's
  "only show when relevant" system-tray mode hides the icon and restores it
  when the headset comes back.

## 2.0.5 — 2026-11-04

- Fixed blurry 22/24 px digits by using a crisp one-pixel seven-segment face
  at those sizes, which are the sizes KDE's tray actually requests.
- Kept the transparent, light/dark adaptive text.

## 2.0.4 — 2026-11-04

- Removed the black circular icon plate; the tray icon now has a transparent
  background.
- Text colour adapts to KDE light/dark window backgrounds.
- 16 px uses a crisp one-pixel 5x7 face; larger sizes use Noto Sans Medium.

## 2.0.3 — 2026-11-04

- Thinner, crisper tray digits: crisp 5x7 pixel digits at 16–24 px and
  Noto Sans Medium at larger sizes.
- Removed the oversized bold font from the compact tray icon.

## 2.0.2 — 2026-11-04

- Rewrote the README for end users: install, usage, troubleshooting and bugs.
- Moved releases to GitHub Actions.  A pushed `v*` tag now builds the source,
  binary and Arch packages, creates the GitHub release, and updates AUR.

## 2.0.1 — 2026-11-04

- Fixed 16–24 px tray icons: use a compact dark badge with large white digits
  instead of a ring that left no room for the percentage.
- Kept the battery ring for 32 px and larger icons.

## 2.0.0 — 2026-11-04

- Rewrote the app in Rust with a raw StatusNotifierItem implementation.
  No Qt, GTK, Electron or web runtime is loaded.
- Tray-only UI: battery ring/tooltip, sidetone submenu, low-battery alerts,
  refresh and autostart.
- Embedded bold font subset for readable two-digit battery text at tray sizes.
- Changed the SNI category to `ApplicationStatus` so KDE groups it with normal
  applications instead of “Hardware”.
- Added CLI: `--battery`, `--probe`, `--set-sidetone`, `--device`.
- Single-instance lock, udev `uaccess` rule, and a pseudo-terminal test suite.
- Measured idle footprint: ~11.3 MiB RSS / ~8.8 MiB PSS, ~0.1 % CPU.
- Added Arch/AUR packaging and GitHub release artifacts.

## 1.0.0

- Initial Linux implementation (C++20 + Qt 6 Widgets), hardware-tested battery
  and sidetone.
