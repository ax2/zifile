---
title: Stage 1 work log
description: Alpha archive core, desktop flows, validation, and remaining evidence.
---

## Goal

Safely browse, create, test, and extract common archives through one Rust core, responsive Windows UIs, an isolated Worker, and evidence-backed distribution artifacts.

## 2026-08-24

### Findings and implementation

Provider review selected replaceable permissive backends for ZIP, 7z, TAR, gzip, zstd, xz, bzip2, lz4, and Brotli. The initial review held RAR outside 1.0 pending licensing and hostile-input evidence; the 2026-08-25 RAR beta section below records the later evidence-backed reversal. Archive parsing was recognized as an untrusted-input boundary requiring process isolation rather than only background threads.

The core gained signature detection, capability reporting, list/test/create/extract, ZIP64 and AES ZIP, 7z and AES 7z, TAR compositions including TAR + LZMA-alone, single streams, path/link/device-name/collision defenses, safety limits, conflict policies, progress, temporary writes, and cooperative cancellation. Desktop and CLI share the same behavior.

The core capability model now also reports creation input shape. ZIP, 7z, and TAR compositions accept files and directories; gzip, Zstandard, XZ, Bzip2, LZ4, and Brotli require exactly one file. Iced and Dioxus preflight this before the destination dialog, disable invalid creation, and present bilingual recovery guidance.

`zifile-worker.exe` and versioned JSON Lines IPC moved desktop parsing out of process. Windows Job Objects limit one process to 4 GiB and kill on close. A 32-operation FIFO scheduler serves both Iced and the Dioxus/WebView2 accessibility candidate. Windows integration includes associations, taskbar progress, App Execution Alias, x64/ARM64 MSIX and runnable artifacts, and an implemented Explorer command DLL/manifest whose trusted-package activation remains open.

The shared open-file dialog extension list now includes `tar.gz`, `tar.zst`, `tar.xz`, and `tar.bz2`, making supported TAR compositions discoverable through their common compound suffixes.
The packaging policy smoke also locks these four compound suffixes into the core open-extension contract, preventing future drift between the dialog and release checks.

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

Opening a large archive already ran in the isolated Worker and could be forcibly reclaimed by the desktop, but the core List API had no common progress or cooperative-cancellation contract. Backward-compatible `ListOptions` and `list_archive_with_options` now cover ZIP, 7z, RAR 1.3–7, CAB, six TAR compositions, and six single compression streams. Every provider checks cancellation at scan boundaries and advances progress. Formats with a known entry count are determinate; TAR/CAB-style scans use explicit bilingual “Scanning” copy until completion publishes a consistent final total. Single streams also report actually decoded bytes.

Worker List now shares the 100 ms reporter and cancellation listener used by test/create/extract, with its final progress snapshot ordered before `archive_start` and streamed entries. Both desktop UIs therefore receive live status, taskbar progress, and their existing Cancel behavior without a protocol change. Core round trips enforce final progress invariants across all 16 format classes and include pre-cancellation; the real Worker smoke parses JSON Lines and checks final-event ordering.

Local verification passed 90 all-workspace/all-target/all-feature Rust tests and three Criterion targets, strict Clippy, rustfmt, the foundation Worker smoke, the 23-script packaging policy, all three nightly fuzz bins, and an Astro build with 27 locale pairs, 55 pages, and zero diagnostics. No foreground UI automation was used while the user session was active; real Narrator and visible-focus evidence remain separate accessibility gates.

The same batch expands the permanent malformed-header regression from ZIP, 7z, TAR, and tar.gz to all 16 format classes. Every minimal input retains enough signature or extension hint to reach its intended provider; List and integrity testing must both return an ordinary error without panicking. Continuous fuzzing and real third-party corpora remain independent defense-in-depth gates.

CAB also gains a decode-stage corruption regression: metadata remains listable while the first compressed CFDATA byte is flipped. Integrity testing and extraction must fail, the atomic temporary file must not commit, and the destination remains empty. This moves fixed coverage beyond header parsing into actual compressed payload and persistence boundaries.

## 2026-08-26 — Modification-time preservation and browsing

ZIP creation now records source file and directory modification times. Safe extraction restores available modification times after atomic file commit for ZIP, 7z, all six TAR compositions, RAR, and CAB; directory times are deferred and applied deepest-first after children so later writes cannot overwrite parent metadata. Round trips cover two files and nested directories, while fixed RAR 5 and CAB fixtures prove the read-only providers restore independently authored timestamps.

Archive listings expose a structured optional timestamp with calendar components, precision, and either UTC or unspecified-offset semantics. The Worker JSON field defaults when absent and is omitted when unknown, preserving protocol-v1 compatibility. Both Iced and Dioxus archive tables show a bilingual Modified column: Unix/NT times are marked UTC, while legacy ZIP/RAR/CAB DOS times explicitly show `no TZ` instead of inventing an offset. Traditional DOS fields remain limited to two-second precision and do not prove the creator's original time zone.

The same shared 100,000-entry view model now sorts by name, original size, packed size, or modified time in either direction. Directories stay first, missing timestamps stay last, ties use a stable path order, and sorting resets to the first page while rendering remains capped at 500 rows. Dioxus table headers are native buttons with `aria-sort`; both UIs show an arrow on the active column. A full 100,000-entry descending-name sort plus bounded-page collection measured 13.96–15.32 ms on the local Windows x64 baseline.

Both desktop UIs now use a shared hierarchical folder view. It synthesizes navigable folders when an archive omits explicit directory entries, exposes root-to-current breadcrumbs, and shows only direct children when search is empty. Search remains archive-wide and displays full paths, while folder and search pages remain capped at 500 rows.

Folder checkboxes recursively select or clear all real descendant files while the folder name remains the navigation control. One archive scan aggregates selected/total counts for every direct child folder, and the Dioxus candidate reports partial selection as `aria-checked="mixed"`. If a malicious archive uses one path as both a file and an implicit directory, the hierarchical view keeps the navigable directory instead of emitting duplicate rows.

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

## 2026-08-29 — Explorer folder-background command

### Change

- Registered “Create archive with ZiFile” for the Windows 11 Explorer `Directory\Background` item type. When invoked on a folder background, Explorer supplies the current folder and the existing Rust COM command opens the create page through the same `--create` protocol; archive work remains in the desktop and isolated Worker.

### Verification

- MSIX manifest audit and packaging-policy gates now require all three create contexts: `*`, `Directory`, and `Directory\Background`. The development package is still unsigned, so real Explorer activation and lifecycle evidence remain part of the trusted-install gate.
- Earlier foreground diagnostics are retained as historical evidence: the default Iced window acquired a foreground handle but exposed no UI Automation `Document` text, and one Dioxus attempt lacked host foreground ownership. Those runs stopped safely before business assertions and do not supersede the successful run below.
- Worker now has a test-only `ZIFILE_TEST_WORKER_DELAY_MS` seam, capped at 10 seconds with cancellation still observed during the delay. The queue script defaults to the 10-second startup window and reacquires the UI Automation button after each redraw, avoiding disk-speed races and stale element references.
- On the current host, `zifile-desktop-accessible.exe` completed a real foreground round with 100,000 entries and three integrity submissions: `foreground_window_verified=true`, active cancellation, next-operation start, pending-queue clearing, Worker reclamation, and zero partial outputs all passed. Default Iced remains outside the same gate because it exposes no certifiable UI Automation `Document` semantic tree, so the Stage 1 queue item stays open.
- To remove the transient full-table render during large-archive queue handoff, both desktop UIs now clear `busy` only when no next job exists; a queued job starts directly while the lightweight busy view remains active. Source regressions protect this ordering in the default Iced and accessible candidates.
- Detail fix: the Explorer extract command now also recognizes `.epub`, matching the core's ZIP-content inspection behavior without adding `.epub` as a default file association.
- Lifetime fix: the shell DLL now atomically counts COM objects and `LockServer` calls, allowing `DllCanUnloadNow` only when both are zero, with a regression test.
- Background-menu robustness fix: an empty `IShellItemArray` now falls back to the current Explorer folder through `IObjectWithSite`; if no site is available, the command never guesses a path.
- Shell create-source deduplication fix: before launching the desktop, the command merges Explorer paths that differ only by Windows casing or slash direction, avoiding duplicate roots and wasted command-line budget; extract invocation still requires one Explorer item.
- Single-stream output-name fix: the legacy `.lzma` and `.bz` aliases now remove their input suffixes correctly, preserving the original filename stem instead of incorrectly adding `.out`; core regression tests cover canonical suffixes, aliases, case variants, and the unknown-suffix fallback.
- LZMA compatibility fix: because `.lzma` has no universal magic, the extension enters the XZ format family and is then decoded by the pure-Rust `lzma-rust2` LZMA-alone reader; existing safety limits become the decoder memory ceiling, and a fixed 37-byte corpus now covers listing and extraction interoperability.
- TAR + LZMA compatibility fix: `.tar.lzma` now has a distinct format contract, pure-Rust LZMA-alone creation and decoding, and a full list/test/extract/create round trip rather than being mistaken for a single-file stream.
- UI consistency fix: the creation order is centralized in the core `ArchiveFormat::CREATABLE` registry, so the Iced baseline and Dioxus candidate share one fourteen-format menu and the CLI contract smoke prevents provider additions from drifting between UIs.
- Explorer detail fix: when slow state evaluation is allowed, the Shell extract command uses the core signature detector and capability registry. A valid archive can still receive the command after renaming, while an invalid file cannot receive it from a forged `.zip` suffix; ordinary directories named `folder.zip` remain excluded.
- Creation-flow detail fix: the shared UI preflight now detects sources that disappeared before the save dialog and shows bilingual recovery guidance in both desktop interfaces; the Worker keeps the final race-time validation.
- Drag-and-drop consistency fix: the default Iced desktop and Dioxus candidate now share signature-first archive classification, support renamed archives, and retain extension fallback for formats without universal magic or failed probes.
- Drag-and-drop threading fix: both Iced and Dioxus move the header probe off the UI event thread, so slow or remote paths cannot block the window.
- Source deduplication fix: both desktop variants now share Windows path-identity comparison, so file pickers, Explorer multi-select, and drag-and-drop cannot submit the same source twice through different casing or slash spellings.
- Windows path detail fix: source identity comparison now uses Unicode lowercase normalization, so non-ASCII filename case variants are deduplicated as well.
- Contract tightening fix: matching extraction-folder naming now follows only the TAR compositions actually supported by the core registry, rather than implying support for `.tar.lz4`, `.tar.br`, or `.tar.bz`.
- Registry consistency fix: core format detection and Explorer matching-folder naming now share one compound TAR suffix registry, so adding an alias cannot update only one path.
- Cancellation semantics fix: creation and extraction check cancellation again before creating parent or destination directories, so an already-cancelled operation leaves no avoidable empty directory.
- Output safety fix: completed creation and extraction outputs now atomically replace the old target from the temporary file instead of deleting it first, preserving the old contents if final replacement fails.

## 2026-08-29 — Interoperability and CLI contract gates

### Changes

- The Windows reference-tool script now covers bidirectional TAR + LZMA-alone: system `tar.exe --lzma` creates an archive that ZiFile tests and extracts, while `tar.exe` extracts a ZiFile-created archive; the script also emits an eight-case JSON artifact containing no user data.
- Added a CLI contract smoke that locks six public commands, fourteen creation formats, all sixteen `formats` capability rows, both bilingual contract documents, and runtime versus syntax error codes; it is wired into Windows CI and packaging policy.
- Corrected the public contract format list to include `tar-lzma` and synchronized the CHANGELOG and testing strategy.

### Verification

- Local Windows reference-tool interoperability passed 8/8; the CLI contract smoke, Foundation smoke, 14 core unit tests, 34 archive integration tests, strict Clippy, packaging policy, and the 63-page Astro build all passed.
- The current Windows `tar.exe` decodes non-ASCII TAR filenames using the active Windows code page in the reverse TAR+LZMA case, so that direction strictly checks ASCII file content; the reference-to-ZiFile direction still checks Unicode paths, as do the ZIP, tar.gz, and 7z cases.
