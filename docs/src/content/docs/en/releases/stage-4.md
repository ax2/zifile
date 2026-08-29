---
title: Stage 4 work log
description: ZiFile 1.0 preparation, contract freeze, and cross-channel release gates.
---

## Goal

Freeze the public CLI/provider contracts, finish user and release documentation, obtain signing and certification evidence, and publish GitHub, WinGet, and Microsoft Store 1.0 from one version source.

## Current status

Stage 4 is not complete. A public-contract candidate, the 1.0 readiness manifest, the Release workflow, SBOMs, provenance, checksums, and stage records exist, but they prove release preparation rather than completion of every stable-release gate.

## Prepared

- `docs/src/content/docs/en/development/contracts.md` defines CLI commands, format values, conflict policies, password input, exit codes, core-provider boundaries, and Worker protocol compatibility.
- `tests/smoke/contract-policy.ps1` now wires the candidate CLI commands, fifteen creation formats, seventeen capability rows, bilingual contract pages, and exit codes into Windows CI; the final freeze remains reserved for the 1.0 release commit.
- `release/readiness.json` binds stable tags to accessibility, queue, trusted-install, ARM64, screenshot, WACK, WinGet, Store, Partner Center, and signing evidence.
- The Release workflow derives dual-architecture builds, audits, checksums, SBOMs, provenance, and GitHub Releases from the workspace version; ordinary public releases use unsigned artifacts, while formal signing can be enabled explicitly through workflow inputs.
- `v0.1.0-alpha.1` is publicly available as a prerelease; the workspace is now set to `v0.1.0`, whose public GitHub Release will be generated from the matching tag.

## Required to finish

- Close the real foreground queue, trusted signed-install lifecycle, physical ARM64, complete accessibility, and WACK gates.
- Complete Partner Center identity, SignPath/production signing, formal bilingual Store screenshots, WinGet acceptance, and Store certification.
- Freeze the CLI, provider, and IPC contracts on the 1.0 release commit, then update final release notes, Stage 4 evidence, and the release result.

## Release result

Stable 1.0 has not been published; `v0.1.0` is the current usable public GitHub version, while formal Store, WinGet, and trusted-signing gates remain incomplete. Stage 4 remains active.

## 2026-08-29 quality closeout

- Both desktop archive tables now provide a bilingual `Copy checksum` action: Iced uses the system clipboard, the accessible Dioxus UI uses the Clipboard API, and missing capability or Promise failure becomes an error state. Copy labels, wiring, and failure fallback are covered by tests.
- Both desktop archive headers now provide a bilingual `Show in File Explorer` action that selects the current archive in its containing folder; launch failures become error status and the path is not persisted.
- The accessible archive browser now uses a lightweight busy summary during high-frequency progress refreshes for large archives, retaining cancel, queue, open-another, and File Explorer actions before restoring the table after completion; this avoids repeatedly scanning 100,000 entries while foreground queue actions are submitted.
- Standalone `.lzma` is now a distinct `ArchiveFormat::Lzma` instead of a read-only alias overloaded onto XZ: the core uses `lzma-rust2` for LZMA-alone read/write, while the CLI, Iced, Dioxus, capability matrix, and Windows Foundation smoke expose a dedicated `lzma` option. `.xz` display and detection no longer conflate the two containers.
- The Explorer create command now rejects disappeared paths, non-file-system virtual items, and symbolic links at the Shell menu boundary while retaining command-line budget and path-deduplication checks; Shell regression coverage is now 19 tests, including real file/directory acceptance and stale-source rejection.
- Core extraction now rejects symbolic links, junctions, and reparse points in the destination itself and existing output parents, with a regression proving that no file is written through the link target; archive-entry link rejection and host-destination checks now share the same boundary.

- The locale check now requires all 31 Chinese/English page pairs to exist and have a non-empty body after front matter is removed; a present-but-blank Markdown page fails the check.
- `cargo test --workspace --all-targets --all-features --locked` passed across the CLI, core, 42 archive regressions, Iced, accessible candidate, Shell, Worker, protocol, and Criterion targets.
- The Astro static build generated 63 pages with 0 errors/warnings/hints; user documentation, Node/PowerShell syntax, packaging policy, and worktree hygiene checks passed.
- This is local code and documentation evidence. It does not change the 11 external release gates from `pending` and does not replace trusted signing, Store/WinGet, physical ARM64, WACK, or real assistive-technology validation.
