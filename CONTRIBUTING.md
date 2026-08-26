# Contributing to ZiFile

ZiFile accepts focused issues and pull requests. Archive inputs are untrusted,
the desktop has two Rust UI implementations during the accessibility migration,
and Windows packaging has release-specific gates. Please open a design issue
before a large format, security, UI, IPC, or packaging change.

The detailed contributor guide is available in
[English](docs/src/content/docs/en/development/contributing.md) and
[Simplified Chinese](docs/src/content/docs/development/contributing.md).

## Toolchain and setup

- Windows 10/11 is the primary product environment.
- Rust is pinned by `rust-toolchain.toml`; do not silently use a different stable toolchain.
- Use the committed `Cargo.lock` and `docs/pnpm-lock.yaml`.
- Install documentation dependencies with `pnpm --dir docs install --frozen-lockfile`.

## Local checks

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features --locked
./tests/smoke/foundation.ps1 -SkipDesktopLaunch
./tests/smoke/packaging-policy.ps1
pnpm --dir docs install --frozen-lockfile
pnpm --dir docs build
```

Run the narrowest relevant interoperability, performance, accessibility, or
packaging script in addition to this baseline. Do not run a foreground UI test
unless the current desktop session is available for it; those scripts must keep
their foreground-window protection.

## Change requirements

- Provider changes need round-trip or read-only corpus coverage, hostile-input
  limits, cancellation behavior, and capability-registry updates.
- Shared desktop behavior must remain consistent in the Iced baseline and the
  Dioxus/WebView2 accessibility candidate until the default UI changes.
- User-visible behavior needs matching Simplified Chinese and English copy,
  tests, and paired Starlight pages when documentation changes.
- Public CLI/provider/IPC changes must follow the documented compatibility policy.
- Notable behavior and release-process changes update `CHANGELOG.md`; do not cut
  a versioned section until preparing the corresponding tag.
- Changes that complete or add a 1.0 blocker update `release/readiness.json`;
  never mark a gate passed without an authoritative evidence URL.
- Architecture decisions require an ADR under `docs/src/content/docs/architecture/`.
- Never commit passwords, tokens, private keys, signing files, cookies, customer
  archives, or other credentials. Follow [SECURITY.md](SECURITY.md) for vulnerabilities.

## Pull requests

Keep unrelated worktree changes out of the commit, describe evidence boundaries,
and do not claim signed installation, WACK, Store, WinGet, ARM64 runtime, or
accessibility certification from a narrower build or static check. Generated
Windows stage output retains runnable directories and EXEs, not ZIP files, unless
a ZIP is explicitly requested.

All contributions are accepted under the repository's MIT license.
