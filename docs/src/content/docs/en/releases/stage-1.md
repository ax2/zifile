---
title: Stage 1 work log
description: Alpha archive core, desktop flows, validation, and remaining evidence.
---

## Goal

Safely browse, create, test, and extract common archives through one Rust core, responsive Windows UIs, an isolated Worker, and evidence-backed distribution artifacts.

## 2026-08-24

### Findings and implementation

Provider review selected replaceable permissive backends for ZIP, 7z, TAR, gzip, zstd, xz, bzip2, lz4, and Brotli. The initial review held RAR outside 1.0 pending licensing and hostile-input evidence; the 2026-08-25 RAR beta section below records the later evidence-backed reversal. Archive parsing was recognized as an untrusted-input boundary requiring process isolation rather than only background threads.

The core gained signature detection, capability reporting, list/test/create/extract, ZIP64 and AES ZIP, 7z and AES 7z, TAR compositions, single streams, path/link/device-name/collision defenses, safety limits, conflict policies, progress, temporary writes, and cooperative cancellation. Desktop and CLI share the same behavior.

The core capability model now also reports creation input shape. ZIP, 7z, and TAR compositions accept files and directories; gzip, Zstandard, XZ, Bzip2, LZ4, and Brotli require exactly one file. Iced and Dioxus preflight this before the destination dialog, disable invalid creation, and present bilingual recovery guidance.

`zifile-worker.exe` and versioned JSON Lines IPC moved desktop parsing out of process. Windows Job Objects limit one process to 4 GiB and kill on close. A 32-operation FIFO scheduler serves both Iced and the Dioxus/WebView2 accessibility candidate. Windows integration includes associations, taskbar progress, App Execution Alias, x64/ARM64 MSIX and runnable artifacts, and an implemented Explorer command DLL/manifest whose trusted-package activation remains open.

The Dioxus candidate added semantic browse/create flows, CSP and offline policy, native drop, shortcuts, bilingual persisted UI, UI Automation-visible controls, bounded 500-row paging, and foreground-protected bilingual keyboard regression. It remains opt-in until Narrator, Accessibility Insights, visible focus, high contrast, IME, DPI, physical ARM64, and real drop gates pass.

### Verification

Strict format/Clippy/tests, benchmark compilation, dependency policy, Worker cancellation, package policy, and Windows ZIP/tar.gz/7z bidirectional interoperability passed repeatedly in CI. Dual-architecture Release exercises produced MSIX, runnable directories, standalone EXEs, checksums, audits, CycloneDX SBOMs, and provenance without ZIP release artifacts. Candidate and default PE machine types and checksums were independently verified.

Real Windows runs covered 1,200-entry paging and a deterministic 100,000-entry ZIP. The candidate exposed only the current 500 rows, searched the final item, navigated filtered pages, measured same-instant process-tree memory, and completed five cancellation runs with the Worker reclaimed. Keyboard runs covered bilingual navigation and the create form while refusing to send input if ZiFile lost foreground focus.

WinGet 1.12 candidate manifests generated from real Release hashes passed local validation. Development MSIX and one-day self-signed exercises proved package/signing wiring but did not establish trusted installation. No test private key, certificate, trusted root, or package registration was retained.

### Remaining work

- Real foreground queued-operation smoke, continued malformed/bomb/libarchive corpus expansion, and trusted Explorer activation.
- Signed install/upgrade/repair/uninstall and WACK in suitable interactive environments.
- Complete archive/extract keyboard traversal, Narrator, Accessibility Insights, visible focus, high contrast, Chinese IME, per-monitor DPI, real drop, and physical ARM64 execution.
- Partner Center identity/name, production signing, Store submission, and WinGet PR.
- Signed-package reproducibility and cross-machine/toolchain evidence remain separate gates. Raw PE reproducibility was completed later in this log: run `32826187552` produced 5/5 on both x64 and ARM64, with later clean-merge runs preserving 5/5.

## 2026-08-26 — Archive-open progress and cancellation

Opening a large archive already ran in the isolated Worker and could be forcibly reclaimed by the desktop, but the core List API had no common progress or cooperative-cancellation contract. Backward-compatible `ListOptions` and `list_archive_with_options` now cover ZIP, 7z, RAR 1.3–7, CAB, five TAR compositions, and six single compression streams. Every provider checks cancellation at scan boundaries and advances progress. Formats with a known entry count are determinate; TAR/CAB-style scans use explicit bilingual “Scanning” copy until completion publishes a consistent final total. Single streams also report actually decoded bytes.

Worker List now shares the 100 ms reporter and cancellation listener used by test/create/extract, with its final progress snapshot ordered before `archive_start` and streamed entries. Both desktop UIs therefore receive live status, taskbar progress, and their existing Cancel behavior without a protocol change. Core round trips enforce final progress invariants across all 15 format classes and include pre-cancellation; the real Worker smoke parses JSON Lines and checks final-event ordering.

Local verification passed 90 all-workspace/all-target/all-feature Rust tests and three Criterion targets, strict Clippy, rustfmt, the foundation Worker smoke, the 23-script packaging policy, all three nightly fuzz bins, and an Astro build with 27 locale pairs, 55 pages, and zero diagnostics. No foreground UI automation was used while the user session was active; real Narrator and visible-focus evidence remain separate accessibility gates.

The same batch expands the permanent malformed-header regression from ZIP, 7z, TAR, and tar.gz to all 15 format classes. Every minimal input retains enough signature or extension hint to reach its intended provider; List and integrity testing must both return an ordinary error without panicking. Continuous fuzzing and real third-party corpora remain independent defense-in-depth gates.

CAB also gains a decode-stage corruption regression: metadata remains listable while the first compressed CFDATA byte is flipped. Integrity testing and extraction must fail, the atomic temporary file must not commit, and the destination remains empty. This moves fixed coverage beyond header parsing into actual compressed payload and persistence boundaries.

## 2026-08-26 — Modification-time preservation and browsing

ZIP creation now records source file and directory modification times. Safe extraction restores available modification times after atomic file commit for ZIP, 7z, all five TAR compositions, RAR, and CAB; directory times are deferred and applied deepest-first after children so later writes cannot overwrite parent metadata. Round trips cover two files and nested directories, while fixed RAR 5 and CAB fixtures prove the read-only providers restore independently authored timestamps.

Archive listings expose a structured optional timestamp with calendar components, precision, and either UTC or unspecified-offset semantics. The Worker JSON field defaults when absent and is omitted when unknown, preserving protocol-v1 compatibility. Both Iced and Dioxus archive tables show a bilingual Modified column: Unix/NT times are marked UTC, while legacy ZIP/RAR/CAB DOS times explicitly show `no TZ` instead of inventing an offset. Traditional DOS fields remain limited to two-second precision and do not prove the creator's original time zone.

The same shared 100,000-entry view model now sorts by name, original size, packed size, or modified time in either direction. Directories stay first, missing timestamps stay last, ties use a stable path order, and sorting resets to the first page while rendering remains capped at 500 rows. Dioxus table headers are native buttons with `aria-sort`; both UIs show an arrow on the active column. A full 100,000-entry descending-name sort plus bounded-page collection measured 13.96–15.32 ms on the local Windows x64 baseline.

Both desktop UIs now use a shared hierarchical folder view. It synthesizes navigable folders when an archive omits explicit directory entries, exposes root-to-current breadcrumbs, and shows only direct children when search is empty. Search remains archive-wide and displays full paths, while folder and search pages remain capped at 500 rows.

## 2026-08-24 — Parser-boundary hardening

Extraction originally listed with global defaults before applying caller limits, so strict caller entry/depth limits did not constrain early parser work. Limit-aware list/test APIs now apply extraction limits before destination creation. Integration tests cover strict ZIP/7z/TAR limits and malformed ZIP/7z/tar/tgz inputs.

An `archive_parsers` libFuzzer target covers all 13 supported archive/stream signatures with bounded input, time, RSS, entry count, expansion, ratio, and path depth. Linux GNU is the dynamic campaign environment because the Windows/MSVC fuzz entry point conflicts with the 7z DLL link model; Windows still compiles the target as Rust code.

Campaign `32733658052` found a 292-byte malformed 7z that triggered a `capacity overflow` in `sevenz-rust2` 0.20.2. Campaign `32803785688` found a 173-byte input that requested an ASan-scale allocation, proving panic catching alone cannot contain OOM. Both exact artifacts became permanent integration fixtures and mandatory campaign-start replays.

ZiFile upgraded to Rust 1.93.0 and `sevenz-rust2` 0.22.0, whose bounded metadata counts reject both inputs as ordinary backend errors. Provider unwind boundaries remain defense in depth; OOM and process termination remain Worker/fuzz concerns. Targeted run `32813469578` replayed both fixtures, then executed 498,937 more inputs over 181 seconds with 370 MiB peak RSS and no new artifact. CI `32813453887` passed all quality, Worker, interoperability, documentation, dependency, benchmark, and fuzz-compile gates.

Double-build run `32813453959` remained 4/5 identical on both x64 and ARM64. Schema-v2 run `32822543635` proved that both default Iced executables first differed at the `build-a`/`build-b` target path embedded in generated `glutin_wgl_sys` code. After remapping each isolated root to `Z:\zifile-target`, run `32826187552` produced 5/5 and `reproducible=true` on both architectures. This remains an Alpha checkpoint, not a Store-ready release.

The CLI removed plaintext `--password <value>` process arguments in favor of explicit `--password-stdin`. Three unit tests and the foundation smoke cover non-empty single-line input, space preservation, the help-surface policy, and real AES 7z create/test/extract operations.

An official 7-Zip bidirectional corpus gate defines seven reference-created codec/filter/encryption cases and two ZiFile-created cases, with complete file-set and SHA-256 comparison plus uploaded JSON evidence. The local machine has no `7z.exe`; GitHub Windows CI supplies the independent reference tool.

The first cloud run, `32835391711`, reached the Deflate case and exposed that the optional `sevenz-rust2` Deflate decoder feature was not enabled by its defaults. ZiFile now enables that feature explicitly; corrected run `32836336921` completed all nine cases.

The corrected [CI run 32836336921](https://github.com/ax2/zifile/actions/runs/32836336921) passed all four jobs. Seven reference-created and two ZiFile-created cases passed complete file-set and SHA-256 comparison with 7-Zip 26.02. The evidence JSON SHA-256 is `06278BB8B96AB683A3C117BA5E30F1B4AB1CF89F1BBF01E72BAC0CC26B49DB14`.

A trusted-signed MSIX lifecycle script now audits baseline and upgrade packages, refuses to overwrite an existing installation, checks install, packaged CLI, upgrade and Reset, and guarantees uninstall cleanup plus JSON evidence. No formally signed packages are available yet, so the mutating lifecycle has not been run; Reset and data-preserving Repair remain distinct claims.

The trusted-signature precondition was exercised with a structurally current unsigned x64 development MSIX. Audit rejected `NotSigned`, and the `ZiCode.ZiFile.Dev` installed-package count remained zero before and after, proving that package registration was untouched.

## 2026-08-25 — Pure-Rust read-only RAR beta

The earlier RAR hold was revisited after `rars` 0.9.3 provided a pure-Rust, `unsafe`-forbidden MIT OR Apache-2.0 implementation covering RAR 1.3 through RAR 7. ZiFile now reports browse/test/extract and encrypted-read capabilities while continuing to reject RAR creation.

Core integration covers every archive version exposed by the provider, solid selective extraction, Unicode names, encrypted headers, wrong/missing passwords, strict limits, cancellation, temporary-file commit semantics, links, reparse attributes and RAR 5+ redirections. The parser fuzz selector includes RAR and MSIX associates `.rar`. CI `32853686537` passed six valid pinned external fixtures and three unsafe-link/redirection rejection cases. RAR 1.3 is compared with the known-good tree from the pinned upstream commit because current 7-Zip no longer reads that legacy version; the other five valid cases are cross-checked against 7-Zip 26.02. The evidence JSON SHA-256 is `4C52D0240B911609C7DDB0CACB2E484F56C8F886E216347603B228261C4EE8EF`.
