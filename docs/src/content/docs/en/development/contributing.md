---
title: Contributing
description: ZiFile development setup, change requirements, evidence, and pull-request rules.
---

## Before you start

ZiFile treats archive input as untrusted. The Windows desktop maintains an Iced baseline and a Dioxus/WebView2 candidate during the accessibility migration, and packaging has separate release gates. Open a design issue before a large format, security, UI, IPC, or distribution change; focused fixes can go directly to a focused pull request.

Windows 10/11 is the primary product environment. `rust-toolchain.toml` pins Rust, while `Cargo.lock` and `docs/pnpm-lock.yaml` lock product and documentation dependencies. Do not silently substitute another toolchain or unlocked dependency set.

## Baseline gates

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features --locked
./tests/smoke/foundation.ps1 -SkipDesktopLaunch
./tests/smoke/packaging-policy.ps1
pnpm --dir docs install --frozen-lockfile
pnpm --dir docs build
```

Also run the narrowest relevant interoperability, performance, accessibility, or packaging script. Real-window scripts run only on an available interactive desktop and retain foreground-window protection. Compilation or static UI Automation cannot substitute for real Narrator, IME, DPI, high-contrast, or cross-window drag-and-drop evidence.

## Change requirements

- Archive providers update the capability registry and add round-trip or read-only corpus coverage, resource limits, hostile-input behavior, cancellation, and temporary-output evidence.
- Shared desktop workflows cover both the Iced baseline and Dioxus candidate until the default UI changes. Accessibility changes also cover keyboard, focus, names, status, and announcement boundaries.
- User-visible copy stays synchronized in Simplified Chinese and English. Starlight pages use matching locale paths.
- CLI, core-provider, and IPC changes follow [Public contracts and version policy](/zifile/en/development/contracts/).
- Notable feature or process changes update `CHANGELOG.md`; cut a dated version section only when preparing its matching tag.
- New architecture decisions add an ADR under `docs/src/content/docs/architecture/`.

## Security and evidence

Never commit passwords, tokens, cookies, private keys, signing files, customer archives, or real sensitive data. Report vulnerabilities privately as described by the root `SECURITY.md`. Pull-request text distinguishes implementation, local evidence, cloud evidence, and remaining device/account/certification gates. An unsigned package, static manifest, or readiness result is not proof of trusted installation, WACK, Store, or WinGet acceptance.

Stage only files belonging to the change and preserve other worktree edits. Windows stage artifacts retain complete runnable directories and EXEs, not ZIP files, unless a task explicitly requires ZIP.
