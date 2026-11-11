# Security policy

GSP 370 Tray is a local desktop application. It opens a USB serial device,
writes small user-owned configuration files and talks to the desktop over
D-Bus. It has no network service and no remote API.

## Supported versions

Security fixes are provided for the latest tagged release. Older releases may
not receive backports; users should upgrade through their package manager or
the latest GitHub release.

| Version | Supported |
|---|---|
| 2.2.x | Yes |
| Earlier | No |

## Reporting a vulnerability

Please do not open a public issue for a security problem. Use GitHub's private
vulnerability reporting for this repository:

1. Open the repository's **Security** tab.
2. Choose **Advisories** and **Report a vulnerability**.

If private reporting is unavailable, open a minimal public issue asking for a
private contact channel; do not include exploit details, credentials or
personal data in that issue.

Include, where relevant:

- the affected version (`gsp370-tray --version`);
- the installation method (AUR, prebuilt archive or source);
- a description of the impact and the conditions required to trigger it;
- a minimal reproduction or proof of concept;
- any suggested fix.

Reports are handled on a best-effort basis. There is no bug bounty program.

## Scope

Examples of issues in scope:

- bypassing the `/dev` restriction for `--device` or `GSP370_TTY`;
- writing outside the intended config, autostart, documentation or state paths;
- privilege escalation through the source installer or packaged udev rule;
- unintended access to the serial device or its data;
- vulnerabilities in the release or AUR automation that could alter published
  artifacts.

General support questions, missing features and device compatibility problems
belong in the [issue tracker](https://github.com/NasirA1991/gsp370-tray/issues),
not in a security report.

## Operational notes

- The AUR build uses the source archive attached to the tagged GitHub release
  and verifies its SHA-256 checksum.
- The release workflow uses a repository secret for the AUR SSH key. Never
  paste that key, tokens or other credentials into an issue, pull request or
  log.
- The udev rule grants access to the active local seat and sets a `uucp` group
  fallback; it does not make the device world-readable.
- Running the application as root is not required and is not supported for
  normal tray use.
