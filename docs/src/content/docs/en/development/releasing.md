---
title: Build and release
description: One versioned release flow for GitHub, WinGet, and Microsoft Store.
---

## Artifacts and channels

Releases contain x64 and ARM64 desktop/CLI programs, MSIX packages, standalone executables, SHA-256 files, per-package audit JSON, CycloneDX JSON SBOMs, GitHub build provenance, release notes, and a matching documentation snapshot. GitHub Release is the first public channel. WinGet uses planned ID `ZiCode.ZiFile`; Microsoft Store uses MSIX.

A `v*` tag builds both architectures and WinGet 1.12 multi-file manifest candidates. Tag publishing requires the official Identity, Publisher, PFX, and password secrets. Missing inputs, `.Dev` identities, or Microsoft's unsigned-package OID are rejected before build. Without official credentials, only manual unsigned development artifacts may be produced and they must not be submitted to WinGet or Store.

Each MSIX is unpacked and audited for identity, publisher, version, minimum Windows build, PE architecture of desktop/CLI/Worker/Explorer DLL, associations, `zifile.exe` alias, absence of sensitive files and ZIP artifacts, and signature state. Audit output does not replace real install, upgrade, uninstall, or WACK testing.

`tests/smoke/msix-lifecycle.ps1` provides an explicit machine gate for trusted signed baseline and upgrade packages. It audits both packages, refuses any pre-existing installation with the same identity, then checks installation, the packaged CLI, version upgrade, `Reset-AppxPackage`, and guaranteed cleanup with JSON evidence. Microsoft defines Reset as restoring initial configuration, so the script does not label it as a data-preserving Repair operation; formal Repair remains a separate gate. See the [Appx module](https://learn.microsoft.com/powershell/module/appx/) and [Reset-AppxPackage](https://learn.microsoft.com/powershell/module/appx/reset-appxpackage).

The manual **Trusted MSIX lifecycle** workflow accepts two Release validation run IDs, reads the default package and audit JSON from each `windows-x64` artifact, and executes the same gate on a clean Windows Runner with 30-day evidence retention. Real ARM64 installation remains a physical ARM64 Windows gate.

Windows Release uses pinned Rust 1.93.0, `Cargo.lock`, a single Cargo job, and MSVC `/Brepro`. The separate double-build gate compares five raw PE files; see [Reproducible Windows builds](/zifile/en/development/reproducible-builds/).

Before tagging, the Release workflow can be run manually with a semantic version. It saves real dual-architecture artifacts and SBOMs but skips public publishing. Manual validation also packages the `-accessible` candidate; tags continue to publish only the default UI until accessibility and physical-architecture gates pass.

Stable releases require unit, interoperability, security, performance, install/upgrade, and documentation checks plus synchronized Stage logs. WACK must run in an administrator's interactive session.
