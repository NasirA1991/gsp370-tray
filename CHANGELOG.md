# Changelog

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
