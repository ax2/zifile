---
title: Reproducible Windows builds
description: Pinned tools, deterministic linking, and double-build SHA-256 comparison.
---

ZiFile distinguishes “can be built again” from byte-for-byte identical output. `rust-toolchain.toml` pins Rust 1.93.0. Windows Release uses one Cargo job, MSVC `/Brepro`, committed `Cargo.lock`, and `--locked`. The double-build script also uses `CARGO_ENCODED_RUSTFLAGS` to map each isolated `CARGO_TARGET_DIR` to `Z:\zifile-target` with `--remap-path-prefix`, preventing generated Rust sources from embedding the distinct `build-a` and `build-b` roots in panic locations.

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

Cloud run `32813453959` on Rust 1.93.0 produced 4/5 identical PE files on both x64 and ARM64. Schema-v2 run `32822543635` then identified the same cause on both architectures: the first `.rdata` difference was the `build-a` versus `build-b` isolated target path embedded in generated `glutin_wgl_sys` bindings. Checked code/data/resource sections otherwise matched; the header changed because `/Brepro` derives its value from differing content. After path remapping, [run `32826187552`](https://github.com/ax2/zifile/actions/runs/32826187552) produced `reproducible=true` and 5/5 matching PE files on both x64 and ARM64, completing the same-Runner/source/toolchain raw-PE gate.

This gate covers raw PE bytes on one Windows Runner, source revision, and locked toolchain. Signatures and MSIX container metadata require separate double-package checks, and cross-machine/MSVC-version reproducibility remains unproven. A 4/5 result must never be described as fully reproducible.
