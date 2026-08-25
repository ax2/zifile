---
title: Reproducible Windows builds
description: Pinned tools, deterministic linking, and double-build SHA-256 comparison.
---

ZiFile distinguishes “can be built again” from byte-for-byte identical output. `rust-toolchain.toml` pins Rust 1.93.0. Windows Release uses one Cargo job, MSVC `/Brepro`, committed `Cargo.lock`, and `--locked`.

## Local verification

```powershell
./tests/reproducibility/windows-build.ps1 -Architecture x64
```

Two isolated target directories build all Release features and compare `zifile-desktop.exe`, `zifile-desktop-accessible.exe`, `zifile.exe`, `zifile-worker.exe`, and `zifile_shell.dll`. `target/reproducibility-x64.json` records source state, compiler, target, commands, and hashes. Schema v2 records the first difference and bounded 64-byte hex context globally and for every differing PE component, plus ranges and hashes for headers, sections, and overlay. Any mismatch fails; temporary build directories are safely removed.

Existing PE files can be compared without rebuilding:

```powershell
./tests/reproducibility/windows-build.ps1 `
  -ComparePeFirstPath first\zifile-desktop.exe `
  -ComparePeSecondPath second\zifile-desktop.exe
```

Use `-Architecture arm64` for ARM64. Monthly, manual, and build-affecting PR runs retain JSON evidence for 30 days.

## Current evidence and boundary

Cloud run `32813453959` on Rust 1.93.0 produced 4/5 identical PE files on both x64 and ARM64. Only the default Iced executable differed, so the overall gate correctly failed and the roadmap remains open. A later schema-v2 run showed matching `.text`, `.data`, `.pdata`, `.rsrc`, and `.reloc` with differences in headers and `.rdata`; component-level evidence is being collected to locate the remaining source.

This gate covers raw PE bytes on one Windows Runner, source revision, and locked toolchain. Signatures and MSIX container metadata require separate double-package checks, and cross-machine/MSVC-version reproducibility remains unproven. A 4/5 result must never be described as fully reproducible.
