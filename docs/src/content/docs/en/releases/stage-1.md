---
title: Stage 1 work log
description: Alpha archive core, desktop flows, validation, and remaining evidence.
---

## Goal

Safely browse, create, test, and extract common archives through one Rust core, responsive Windows UIs, an isolated Worker, and evidence-backed distribution artifacts.

## 2026-08-24

### Findings and implementation

Provider review selected replaceable permissive backends for ZIP, 7z, TAR, gzip, zstd, xz, bzip2, lz4, and Brotli. RAR stayed outside 1.0 pending licensing and hostile-input review. Archive parsing was recognized as an untrusted-input boundary requiring process isolation rather than only background threads.

The core gained signature detection, capability reporting, list/test/create/extract, ZIP64 and AES ZIP, 7z and AES 7z, TAR compositions, single streams, path/link/device-name/collision defenses, safety limits, conflict policies, progress, temporary writes, and cooperative cancellation. Desktop and CLI share the same behavior.

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
- Full byte-for-byte Windows reproducibility; four of five raw PE outputs match on each architecture, while the default Iced executable remains under diagnosis.

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
