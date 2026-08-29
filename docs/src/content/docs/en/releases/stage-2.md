---
title: Stage 2 work log
description: ZiFile Beta records for Windows integration, the isolated Worker, and distribution artifacts.
---

## Goal

Complete the Windows daily workflow: associations, drag and drop, taskbar feedback, the isolated Worker, dual-architecture packages, and evidence-backed Explorer integration.

## Evidence scope

This page is reconstructed from the current source, package audits, retained CI/Release results, and the current worktree. Formal signed installation, physical ARM64, and real Explorer lifecycle evidence are missing and are not represented as complete.

## Delivered

- The desktop starts `zifile-worker.exe` through versioned JSON Lines IPC and uses Windows Job Objects for Worker lifetime, memory, and close-time reclamation.
- File associations, the App Execution Alias, taskbar progress, desktop drop handling, and runnable directories use the shared core capability model for ZIP/7z/TAR families and single-stream formats.
- MSIX and standalone EXE artifacts target x64 and ARM64; the build path produces checksums, SBOMs, provenance, and package audits.
- Windows 11 Explorer integration is a pure-Rust `IExplorerCommand` DLL with create and extract commands. The create command covers selected files, folders, and `Directory\Background`; the extract command is shown only for one supported archive.
- The Shell DLL only collects filesystem paths and starts the visible desktop. Parsing, passwords, progress, cancellation, and safety limits remain in the desktop and isolated Worker.

## Verification

- Retained Windows integration CI `32663024457` and dual-architecture Release rehearsal `32663037787` completed dependency, Rust, real Worker smoke, MSIX, SBOM, provenance, and artifact-upload checks.
- Release run `33184684164` successfully produced x64/ARM64 artifacts; the Alpha prerelease path intentionally skipped production signing, Store, and WinGet gates.
- The current local x64 MSIX audit confirms `0x8664` for the desktop, CLI, Worker, and Shell DLL, and confirms `*`, `Directory`, and `Directory\Background` create contexts. The package is `NotSigned`.

## Remaining work

- [#12](https://github.com/ax2/zifile/issues/12): trusted signed install, upgrade, Repair/Reset, uninstall, and Explorer activation/cleanup.
- [#13](https://github.com/ax2/zifile/issues/13): physical ARM64 Windows execution evidence.
- An unsigned development package cannot prove Store readiness or real Explorer lifecycle behavior; manifest presence is not installation evidence.

## Release result

The Beta implementation and auditable non-release artifact chain are in place. A formal Beta completion claim remains open until trusted signing and hardware/lifecycle evidence exist.

## 2026-08-29 — Shell capability convergence and creation preflight

### Changes

- When Explorer permits a slow state query, the Shell extract command now reuses core `detect_format` and format capabilities instead of maintaining a second extension allowlist. Valid renamed archives remain discoverable, invalid files cannot use a forged `.zip` suffix to obtain the command, and Explorer items must still be real files.
- Both desktop UIs now preflight sources that disappeared before the save dialog and show bilingual recovery guidance; the Worker still rechecks sources at execution time.

### Verification

- `cargo test --workspace --all-features --locked` passed; the Shell's 14 tests include the archive-named-directory regression, and strict Clippy passed.
- `Build-Package.ps1 -Version 0.1.0.1 -Architecture x64` produced the runnable directory and development MSIX; the package audit confirms four x64 PE payloads and `*`, `Directory`, and `Directory\Background` Shell contexts. The package remains `NotSigned`. The MSIX SHA-256 is `A9491A363ABFA878D53BF72F964504F89D6E422D272CAA6E0DD2ED6DFEBBD000`.
