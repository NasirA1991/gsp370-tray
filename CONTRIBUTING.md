# Contributing

Thanks for helping with GSP 370 Tray. The project is deliberately small: a
Rust tray for the GSP 370 dongle, an optional Plasma widget, and the packaging
that installs them.

Before starting a larger change, read the [README](README.md),
[Architecture](docs/architecture.md) and [Protocol notes](docs/protocol.md).
The protocol notes explain which commands are evidenced and which are
deliberately unprobed.

## Reporting bugs and requesting features

Use the [issue tracker](https://github.com/NasirA1991/gsp370-tray/issues).
For device problems, include the outputs listed in
[Troubleshooting](docs/troubleshooting.md#collecting-information-for-a-bug-report)
and say whether you installed from the AUR, the prebuilt archive or source.

Please do not include secrets, private keys or unrelated personal information
in an issue. The app does not need any credentials to run.

Feature requests are welcome, but the project intentionally excludes audio
processing, equalizer effects and firmware flashing. See the
[README limitations](README.md#limitations).

## Development setup

Requirements:

- Rust 1.89 or newer, matching `rust-version` in `rust/Cargo.toml`
- Git and a C linker
- Python 3 for the pseudo-terminal tests
- `make` for the convenience targets (optional; the cargo commands below work
  without it)

```bash
git clone https://github.com/NasirA1991/gsp370-tray.git
cd gsp370-tray
cargo build --release --locked --manifest-path rust/Cargo.toml
cargo test --locked --manifest-path rust/Cargo.toml
python3 tests/e2e_pty.py ./rust/target/release/gsp370-tray
```

The same build through the optional Makefile is `make build`.

Run the built binary directly if you do not want to install it:

```bash
./rust/target/release/gsp370-tray
```

The integration tests do not need the real headset. `tests/e2e_pty.py`
emulates the dongle over a pseudo-terminal. See
[Testing](#testing) for details.

## Testing

| Command | What it runs |
|---|---|
| `make test` | Rust unit tests |
| `make e2e` | Builds the release binary and runs the PTY suite |
| `make fmt` | Applies `cargo fmt` (writes changes) |
| `make clippy` | Clippy with warnings denied |
| `make check` | `fmt`, clippy, unit tests and PTY suite |

CI runs the checks from a clean checkout. The equivalent local commands are:

```bash
cd rust
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
cargo check --locked --target aarch64-unknown-linux-gnu
python3 ../tests/e2e_pty.py ./target/debug/gsp370-tray
cd ..
```

The MSRV job uses Rust 1.89.0 with `cargo check --locked`. The audit job runs
`rustsec/audit-check` against the lockfile; warnings about unmaintained
transitive crates are reported for visibility.

Run the tests before opening a pull request. For a serial/protocol change,
explain the hardware or capture evidence in the pull request and update
[docs/protocol.md](docs/protocol.md).

## Code guidelines

- Keep the serial worker blocking and single-threaded; do not introduce an
  async runtime for it.
- Preserve the `--locked` build and the declared Rust 1.89 MSRV.
- Avoid new runtime dependencies unless they are clearly necessary. Explain
  the trade-off in the pull request.
- Keep device paths canonicalised under `/dev`; do not weaken the
  `--device`/`GSP370_TTY` validation.
- Do not send unobserved protocol opcodes. In particular, do not experiment
  with firmware-update or DFU commands.
- Keep user-visible strings, CLI output fields, config keys and status names
  stable unless the change intentionally updates the interface. Update the
  relevant docs when they change.
- Add or update tests for parsing, state handling, CLI and protocol changes.

Run `cargo fmt` before committing. Clippy is treated as an error in CI.

## Pull requests

- Branch from `main`; keep each pull request focused.
- Use a descriptive title and explain the user-visible effect, if any.
- Include the tests you ran. State explicitly if you could not test with real
  hardware.
- Update `CHANGELOG.md` for user-visible changes, and update README/docs when
  instructions, behaviour or limitations change.
- Do not bump the package version or push tags; releases are handled
  separately by the maintainer.
- Do not commit generated build outputs, `target/`, editor state or local
  configuration.

CI must pass before merge. Documentation-only pull requests should still
preserve valid links and commands; see [Documentation](#documentation).

## Documentation

Documentation is part of the change, not an afterthought:

- `README.md` is the entry point; keep it short and link to the detailed docs.
- `docs/installation.md` covers AUR, prebuilt and source installs.
- `docs/usage.md` covers the tray, CLI, config, environment and widget.
- `docs/troubleshooting.md` covers user-visible failures.
- `docs/architecture.md` covers internals and test strategy.
- `docs/protocol.md` covers the reverse-engineered serial protocol and
  evidence.

When changing commands, paths, defaults or exit codes, update every affected
document and verify examples against the implementation. When changing
packaging, keep the installed docs in sync with the repository copies.

## Releases

Releases are tag-driven and are performed by the maintainer:

1. Update the version in `rust/Cargo.toml` and `rust/Cargo.lock`.
2. Move the changelog entry into a dated version section.
3. Commit, push `main`, and push an annotated `v*` tag.
4. The release workflow tests and builds the source, binary and Arch packages,
   publishes the GitHub release, then updates AUR.

The AUR SSH key is a repository secret and must never be committed or printed.
Do not create release assets manually; the workflow produces the source
archive used by AUR.

## Questions

Open an issue on the
[issue tracker](https://github.com/NasirA1991/gsp370-tray/issues). For
security-sensitive reports, follow [SECURITY.md](SECURITY.md) instead of
opening a public issue.
