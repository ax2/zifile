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
- `v0.1.0-alpha.1` is publicly available as a prerelease; `v0.1.0`, `v0.1.2`, `v0.1.3`, `v0.1.4`, and `v0.1.5` have been published as public GitHub Releases from matching tags.

## Required to finish

- Close the real foreground queue, trusted signed-install lifecycle, physical ARM64, complete accessibility, and WACK gates.
- Complete Partner Center identity, SignPath/production signing, formal bilingual Store screenshots, WinGet acceptance, and Store certification.
- Freeze the CLI, provider, and IPC contracts on the 1.0 release commit, then update final release notes, Stage 4 evidence, and the release result.

## Release result

Stable 1.0 has not been published; `v0.1.5` is the current usable public GitHub version, while formal Store, WinGet, and trusted-signing gates remain incomplete. Stage 4 remains active.

## 2026-08-29 public release result

- The [`v0.1.0` Release](https://github.com/ax2/zifile/releases/tag/v0.1.0) is published; its tag and `main` commit `a601738` match, and the Release is neither a draft nor a prerelease.
- The release workflow generated and uploaded x64/ARM64 MSIX packages, desktop/CLI/Worker/Shell runnable files, SHA-256 checksums, SBOMs, audit files, and a WinGet manifest candidate. The Release identifies the build as unsigned GitHub distribution.
- This validates the GitHub distribution path but does not close trusted signing, WinGet community acceptance, Microsoft Store, WACK, physical ARM64, or full accessibility gates.

## 2026-08-29 quality closeout

- Both desktop archive views now provide a bilingual primary `Extract all` action while retaining precise `Extract selected`; archives opened from Explorer continue to extract all contents automatically, so users do not need to select every entry first.
- Both desktop archive tables now provide a bilingual `Copy checksum` action: Iced uses the system clipboard, the accessible Dioxus UI uses the Clipboard API, and missing capability or Promise failure becomes an error state. Copy labels, wiring, and failure fallback are covered by tests.
- Both desktop archive headers now provide a bilingual `Show in File Explorer` action that selects the current archive in its containing folder; launch failures become error status and the path is not persisted.
- The accessible archive browser now uses a lightweight busy summary during high-frequency progress refreshes for large archives, retaining cancel, queue, open-another, and File Explorer actions before restoring the table after completion; this avoids repeatedly scanning 100,000 entries while foreground queue actions are submitted.
- Standalone `.lzma` is now a distinct `ArchiveFormat::Lzma` instead of a read-only alias overloaded onto XZ: the core uses `lzma-rust2` for LZMA-alone read/write, while the CLI, Iced, Dioxus, capability matrix, and Windows Foundation smoke expose a dedicated `lzma` option. `.xz` display and detection no longer conflate the two containers.
- The Explorer create command now rejects disappeared paths, non-file-system virtual items, and symbolic links at the Shell menu boundary while retaining command-line budget and path-deduplication checks; Shell regression coverage is now 19 tests, including real file/directory acceptance and stale-source rejection.
- Core extraction now rejects symbolic links, junctions, and reparse points in the destination itself and existing output parents, with a regression proving that no file is written through the link target; archive-entry link rejection and host-destination checks now share the same boundary.

- The locale check now requires all 31 Chinese/English page pairs to exist and have a non-empty body after front matter is removed; a present-but-blank Markdown page fails the check.
- `cargo test --workspace --all-targets --all-features --locked` passed across the CLI, core, 42 archive regressions, Iced, accessible candidate, Shell, Worker, protocol, and Criterion targets.
- The default Iced window now starts centered and enforces a 920×620 minimum so archive tables, search controls, and creation fields remain usable while resizing; a window-settings regression test covers the contract.
- The Astro static build generated 63 pages with 0 errors/warnings/hints; user documentation, Node/PowerShell syntax, packaging policy, and worktree hygiene checks passed.
- This is local code and documentation evidence. It does not change the 11 external release gates from `pending` and does not replace trusted signing, Store/WinGet, physical ARM64, WACK, or real assistive-technology validation.

## 2026-08-30 — 0.1.2 release preparation

- Workspace, internal crate pins, the documentation package, and `Cargo.lock` are being advanced together to `0.1.2` for a stable patch release containing the all-in-one MSIX and portable executable release assets.

## 2026-08-30 — 0.1.2 release result

- [`v0.1.2 Release`](https://github.com/ax2/zifile/releases/tag/v0.1.2) was published automatically from its tag; the Release is neither a draft nor a prerelease, and workflow [33291234378](https://github.com/ax2/zifile/actions/runs/33291234378) completed successfully.
- Public assets are limited to one all-in-one `msixbundle`, one standalone x64 EXE, one standalone ARM64 EXE, and `SHA256SUMS.txt`; DLLs, build configuration, SBOMs, and the WinGet manifest candidate remain workflow evidence only.

## 2026-08-30 — 0.1.3 standalone portable release

- [`v0.1.3 Release`](https://github.com/ax2/zifile/releases/tag/v0.1.3) was published automatically by workflow [33296532873](https://github.com/ax2/zifile/actions/runs/33296532873) after PR #39 passed all nine CI checks, including x64 and ARM64 reproducible double builds.
- The final public assets are exactly `ZiFile-0.1.3.0-windows.msixbundle`, `zifile-windows-x64.exe`, `zifile-windows-arm64.exe`, and `SHA256SUMS.txt`. The desktop EXE embeds the Worker runtime and starts itself with `--zifile-worker`, so the portable downloads do not require a second Worker file.
- The x64 portable EXE was downloaded from the published Release and matched `SHA256SUMS.txt`; the ARM64 hash is retained in the workflow-generated checksum manifest. The Release remains explicitly unsigned and is not a Store-certified package.
- The ordinary public Release path still produces unsigned GitHub Windows artifacts; SignPath, WinGet community acceptance, Microsoft Store, WACK, physical ARM64, and full assistive-technology gates remain tracked independently. The standalone desktop EXEs are self-contained and do not require a separately downloaded Worker executable.

## 2026-08-30 — 0.1.4 public release result

- [`v0.1.4 Release`](https://github.com/ax2/zifile/releases/tag/v0.1.4) was published automatically by workflow [33315987748](https://github.com/ax2/zifile/actions/runs/33315987748); PR #51 passed version consistency, Rust quality, interoperability, performance, fuzz, and x64/ARM64 reproducible double-build checks.
- The final public assets are exactly `ZiFile-0.1.4.0-windows.msixbundle`, `zifile-windows-x64.exe`, `zifile-windows-arm64.exe`, and `SHA256SUMS.txt`; standalone DLLs, JSON/YAML configuration, SBOM, and provenance files are not published separately.
- The x64/ARM64 Windows jobs, all-in-one MSIX bundle job, SBOM job, and GitHub Release job all succeeded. The signing job was conditionally skipped because SignPath is not configured yet, so this remains an explicitly unsigned GitHub build.

## 2026-08-30 — Public Release asset audit fix

- The stage prerelease Job called the repository-owned `tests/smoke/public-release-assets.ps1` without checking out the tagged commit; on a clean Ubuntu runner this would fail because the script was absent.
- PR [#45](https://github.com/ax2/zifile/pull/45) added repository checkout as the first step of `publish-stage` and added a scoped policy check that keeps the checkout before the public asset audit.
- Merge commit [`4d65881`](https://github.com/ax2/zifile/commit/4d65881c5c20d3a6cb8221d6d98aa94c90d7775a) passed the post-merge main CI [33305272788](https://github.com/ax2/zifile/actions/runs/33305272788).
- The public asset set is unchanged: one all-in-one MSIX, one standalone x64 EXE, one standalone ARM64 EXE, and `SHA256SUMS.txt`; audits, SBOMs, provenance, and WinGet YAML remain workflow artifacts.

## 2026-08-30 — UI validation and WinGet gate resilience

- PR [#48](https://github.com/ax2/zifile/pull/48) surfaces non-empty create-source validation failures directly in the Iced create page with a bilingual danger notice; the Create action remains disabled and a source-level regression guard covers the rendered notice.
- The PR passed `cargo fmt --all -- --check` and `cargo test -p zifile-desktop --all-targets --locked` before merge commit [`1487bf6`](https://github.com/ax2/zifile/commit/1487bf6758d8a69b4844a0a4709e146a13ee0c0a).
- PR [#49](https://github.com/ax2/zifile/pull/49) makes the pinned WinGet validation-client repair retry up to three times with bounded backoff after transient CDN connection failures, and records the attempt limit in the validation evidence.
- All PR #49 checks passed, including official WinGet manifest validation, Rust quality, foundation smoke, performance, fuzz-target compilation, and reference-tool interoperability; merge commit [`cbb6505`](https://github.com/ax2/zifile/commit/cbb6505be639f27b064b98de52c766ebea0ec14d) is now on `main`.

## 2026-08-30 — WinGet all-in-one manifest alignment

- The WinGet generator, verifier, and Release workflow no longer depend on unpublished x64/ARM64 per-architecture MSIX URLs, hashes, or local paths; they accept only the public all-in-one `.msixbundle`.
- The installer manifest retains x64 and ARM64 selection nodes, but the verifier requires both to reference the same bundle URL and SHA-256 and verifies that hash against the local bundle. GitHub and WinGet therefore use the same public installer payload.
- Official `winget validate` with WinGet 1.29.290 accepted the schema 1.12 four-file candidate; all 29 extensions, hash-tamper rejection, and the complete packaging policy passed. Community-repository acceptance and signing remain open gates.

## 2026-08-31 — Default desktop shortcut discoverability

- The default Iced desktop already handled `Ctrl+O` for open, `Ctrl+N` for create, archive-page `Ctrl+A` for select all, `F1` for help, and `Esc` for cancellation, but previously exposed no visible user reference.
- The About pages in both the default Iced UI and accessible candidate now present all five shortcuts and their actions as a bilingual keycap list. A default-UI source regression binds the displayed combinations to the implemented keyboard map so one side cannot silently drift.
- `cargo fmt --all -- --check`, `cargo test -p zifile-desktop --all-targets --all-features --locked`, and full-workspace Clippy pass, covering 32 shared desktop library tests, 37 default application tests, 38 accessible-candidate tests, and six 100,000-entry browser benchmarks. This code-level evidence does not replace real foreground keyboard traversal, Narrator, high-contrast, or visible-focus acceptance.

## 2026-08-31 — Create-password lifecycle closeout

- The default Iced UI and accessible candidate previously retained a create password in form state after the request had already entered execution or the in-memory queue, which was broader than the temporary-retention boundary described by the public privacy statement.
- Both UIs now clear the create-form password immediately only when submission is accepted. A full queue leaves the rejected input available for retry. Worker and queue snapshots continue to release their data on completion, clearing, or exit and never write it to settings or logs.
- New tests cover accepted clearing, rejected retention, and non-create isolation in both implementations. Archive decryption passwords remain scoped to the current archive session so later test and extract operations continue to work.

## 2026-08-31 — Post-completion output discovery

- Both desktop status bars now expose a bilingual `Show output` action: successful creation selects the generated archive in File Explorer, while successful extraction locates its destination directory.
- The path comes only from a `Create` or `Extract` request snapshot and is exposed only for a correctly typed successful result. Starting the next job clears the old path; failures, cancellation, and Worker protocol mismatches cannot display a stale action.
- Request-path and UI-wiring regressions were added to both implementations. The 32 shared desktop library tests, 39 default application tests, 40 accessible-candidate tests, and six 100,000-entry benchmarks pass.

## 2026-08-31 — Main-branch merge gate

- PR #55 exposed an unprotected `main`: requesting auto-merge caused GitHub to merge immediately without waiting for the running CI. The code already had complete local validation and remote checks continued afterward, but the process itself was not release-grade governance.
- `main` now requires a pull request, all seven CI checks against the latest branch, resolved conversations, and linear history. Administrators are also bound; force-pushes and branch deletion are disabled. Required approvals remain zero so a single-maintainer project cannot deadlock on a second account.
- GitHub API readback confirms `strict=true`, `enforce_admins=true`, all seven contexts, `required_approving_review_count=0`, enabled linear history and conversation resolution, and disabled force-pushes/deletion. PR #56 is the first live merge validation under the policy.

## 2026-08-31 — 0.1.5 release preparation

- Create-source validation, all-in-one WinGet manifests, shortcut help, create-password lifecycle handling, and post-completion output discovery accumulated after 0.1.4 now form a useful patch-release boundary.
- The workspace, three internal dependency constraints, six workspace lock entries, and Astro documentation package are aligned at `0.1.5`; the version gate confirms tag `v0.1.5` maps to MSIX `0.1.5.0`.
- `CHANGELOG.md` now follows the standard reverse order with an empty top `[Unreleased]`, then 0.1.5 and older releases. Six 0.1.5 entries pass the tag-ready gate. Download documentation continues to name v0.1.4 until the new Release actually succeeds.

## 2026-08-31 — 0.1.5 release outcome

- PR [#57](https://github.com/ax2/zifile/pull/57) merged after all seven normal quality gates and both x64/ARM64 reproducible double builds passed. Annotated tag `v0.1.5` points exactly to merge commit `8959f3d0042bf9ba29eed299a416bf952821b0c1`.
- [Release workflow 33327132468](https://github.com/ax2/zifile/actions/runs/33327132468) completed workspace tests, both architecture builds, the standalone x64 EXE smoke, ARM64 PE architecture audit, all-in-one MSIX Bundle, attestations, and public publication successfully.
- The non-draft, non-prerelease [v0.1.5 Release](https://github.com/ax2/zifile/releases/tag/v0.1.5) exposes exactly `ZiFile-0.1.5.0-windows.msixbundle`, `zifile-windows-x64.exe`, `zifile-windows-arm64.exe`, and `SHA256SUMS.txt`; no DLL, JSON, YAML, SBOM, or provenance internals are public Release assets.
- All four public files were downloaded again after publication. SHA-256 for the MSIX Bundle and both EXEs matches `SHA256SUMS.txt` and the GitHub asset digests. This proves GitHub publication integrity, not trusted signing, WACK, Store, WinGet, or physical ARM64 execution.

## 2026-08-31 — Archive reload shortcut

- Both desktop UIs already exposed a Reload button, but keyboard users had to traverse to it to refresh an open archive. The conventional file-manager `Ctrl+R` shortcut was neither handled nor discoverable.
- The default Iced UI and accessible candidate now map exact `Ctrl+R` to the existing queued reload path. With no current or pending archive it remains a no-op and never guesses a path. The accessible Reload and Unlock controls expose `aria-keyshortcuts="Control+R"`.
- Bilingual About help and user documentation include the shortcut. Unit regressions in both UIs lock the keyboard map, exact modifiers, visible help, and assistive-technology metadata. Real foreground keyboard and Narrator revalidation remains an external gate.

## 2026-08-31 — Archive search shortcut

- Both desktop UIs now handle exact `Ctrl+F`: with an archive loaded they return to the Archive page and focus its search field while preserving the current query; with no archive they do not intercept the shortcut.
- The accessible candidate exposes `aria-keyshortcuts="Control+F"` and a stable control identifier on the search input. Bilingual About help and desktop documentation describe the action.
- Unit tests cover exact modifiers, the no-archive boundary, visible help, and semantic metadata. Real foreground focus movement and screen-reader revalidation remain external gates.

## 2026-08-31 — Close archive session

- Both desktop headers now expose `Close archive`, with exact `Ctrl+W` support. Closing returns home without exiting the application.
- Closing releases the current archive password, metadata, selection, search, pagination, and folder-navigation state. The action is disabled during active work or queue handoff so a completion cannot write back into a session the user already closed.
- Bilingual About help and desktop documentation describe the action. Tests cover exact modifiers, the busy boundary, session cleanup, visible help, and `aria-keyshortcuts="Control+W"`. Real foreground keyboard and assistive-technology revalidation remain external gates.
