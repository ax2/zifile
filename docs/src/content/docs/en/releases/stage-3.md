---
title: Stage 3 work log
description: ZiFile Release Candidate records for accessibility, documentation, and release gates.
---

## Goal

Prepare a shippable Release Candidate: complete the bilingual UI, accessibility candidate, release automation, Store material, reproducible builds, and public prerelease flow.

## Evidence scope

This page only consolidates implementation and verification evidence already present in the repository. Narrator, Accessibility Insights, formal signing, WACK, Partner Center, WinGet acceptance, and Store certification remain external gates.

## Delivered

- The Dioxus/WebView2 accessibility candidate reuses the Rust core, isolated Worker, taskbar integration, and settings, and provides semantic navigation, archive tables, filtering and paging, selection, testing, extraction, creation, progress, cancellation, drop handling, and core shortcuts.
- The candidate adds local-resource/CSP constraints, bilingual keyboard regression, labelled main-region focus after page changes, dynamic selection semantics, forced-colors styling, and bilingual About/runtime identity.
- The default Iced UI and candidate share format capabilities, folder views, the operation queue, and Worker protocol; Iced remains the default until the full accessibility gates pass.
- Integrity testing now records decoded-content SHA-256 values for regular files; the CLI and both desktop tables expose them, while older Worker events without the optional field remain compatible.
- Starlight documentation, bilingual Store copy, screenshot-manifest validation, WACK readiness, SBOMs, provenance, checksums, and the tag Release workflow are wired into the repository.
- GitHub now has the public Alpha prerelease `v0.1.0-alpha.1`, whose notes include the SignPath Foundation signing statement; the documentation deployment workflow passed.

## Verification

- CI `33184493503` and documentation deployment `33184493619` succeeded; Release `33184684164` completed x64/ARM64 builds, SBOM generation, and prerelease artifact upload.
- Local Rust workspace tests, strict Clippy, packaging policy, x64 MSIX audit, foundation archive smoke, and the Astro build all pass.
- On 2026-08-29, Foundation smoke added and passed real CLI round trips for nine formats: TAR+Zstandard, TAR+XZ, TAR+Bzip2, gzip, Zstandard, XZ, Bzip2, LZ4, and Brotli. Each format was created, integrity-tested, extracted, and checked for expected content.
- On 2026-08-29, core regression coverage confirmed that renamed TAR+gzip, TAR+Zstandard, TAR+XZ, and TAR+Bzip2 files remain identifiable and listable after changing their suffix to `.bin`; both compressed input and decoded header probing are bounded.
- On 2026-08-29, both desktop UIs were aligned on bilingual Worker-error formatting for cancellation, password, unknown-format, destination-conflict, and safety-limit cases while preserving paths and backend diagnostic details.
- On 2026-08-29, creation path regressions rejected output destinations inside the source tree before creating parents or temporary files, preventing self-including archives and empty-directory side effects.
- On 2026-08-29, extraction-path regression covered an existing file used as the destination: core returns a structured destination conflict before directory creation and preserves the original contents.
- On 2026-08-29, creation-path regression covered an existing output file: every temporary-file commit path returns a structured destination conflict and preserves the original contents; extraction's explicit entry-overwrite policy remains unchanged.
- On 2026-08-29, the expansion-ratio guard switched to overflow-free exact arithmetic, with a boundary regression proving that `1000:1` is not weakened by integer-division truncation.
- On 2026-08-29, preference saves moved to flushed and synced temporary-file replacement; Windows uses `MOVEFILE_WRITE_THROUGH`, both desktop UIs surface save failures, and the temporary-directory replacement regression passed.
- On 2026-08-29, single-stream listing was aligned with declared expansion-ratio validation; ratio `0` now strictly permits no decoded output, covered by a gzip regression.
- On 2026-08-29, TAR listing began checking cumulative declared entry sizes against expanded-size and ratio limits before skipping compressed payloads; regressions cover TAR, TAR+gzip, TAR+Zstandard, TAR+XZ, TAR+LZMA, and TAR+Bzip2.
- On 2026-08-29, both desktop UIs moved archive-open, extraction-folder, source-add, and archive-save dialogs away from the UI event thread and added an active-dialog guard against slow-directory stalls and duplicate native windows.
- On 2026-08-29, the default Iced status footer gained `Informational`/`Error` semantics: success, progress, queueing, cancellation, and source-selection updates use the normal style, while Worker failures, create preflight failures, a full queue, preference-save failures, and internal queue errors use the danger theme. Desktop regression tests cover the mapping.
- On 2026-08-29, both desktop UIs began sharing a four-state archive empty-state decision: show opening progress while the Worker is active, an unlock form for password failures, a retry prompt for other open failures, and idle guidance when no archive is pending. Bilingual regressions cover all four states and prevent a loading operation from being reported as failed.
- On 2026-08-29, the Iced search bar gained the same match-count summary as the accessibility candidate. Zero-result searches no longer show an invented “Page 1 / 1”; the page position uses a placeholder, with bilingual formatting covered by shared code and tests.
- On 2026-08-29, a race in the Iced archive-open flow was fixed: a queued or queue-rejected replacement request no longer clears the currently visible archive, and the view switches only when the list Worker actually starts. Two regressions cover queued and full-queue paths.
- On 2026-08-29, both desktop create forms were aligned for single-stream formats: Add files uses a single-file picker, Add folder is rejected in the UI and defensive path, and Dioxus create preflight failures now use error semantics; source-contract regressions cover the divergence points.
- On 2026-08-29, create-format changes were extracted into directly tested state transitions: both UIs clamp format-specific levels, clear inapplicable passwords, and announce bilingual source requirements through the status region.
- On 2026-08-29, a Windows Unicode-entry collision regression used `Ä.txt`/`ä.txt` and required rejection during listing before the filesystem could collapse the case variants.
- On 2026-08-29, both desktop creation forms rejected symbolic-link, junction, and reparse-point sources before the save dialog and provided bilingual guidance to choose the original file or folder.
- On 2026-08-29 the local x64 `0.1.0.10` artifact was rebuilt and passed MakeAppx. The package contains 64 files; desktop, CLI, Worker, and Shell DLL PE machines are all `0x8664`; both Shell CLSIDs, associations, icons, background-menu registration, path deduplication, background-path budget validation, and audit hashes passed. The latest MSIX SHA-256 is `1E5ABC05B9E67252D3B2377F5F3BDBEAC310A3F072DD130EAFAE49E5D0524778`. It uses the development Identity and is unsigned, so it is not trusted-installation evidence.
- On 2026-08-29 the accessible candidate passed a real foreground extraction-cancellation smoke: cancellation occurred during 128 one-MiB entries, bilingual cancellation failure text was observed, every committed file had the complete entry size with zero partial outputs, and zero Worker processes remained.
- On 2026-08-29 the accessible candidate passed the complete bilingual keyboard workflow three consecutive times. Each run covered forward/reverse page navigation, archive test/Reload/search/selection/conflict/paging, the 7z create form with level `6→7→6`, password clearing, and source controls; the fixed test password was never written to evidence.
- On 2026-08-29, the Release workflow was corrected to scope formal Store identity variables: stable tags and explicit cloud-signing rehearsals pass the formal identity, while unsigned prereleases and manual `none` validation retain the `.Dev` identity; the PowerShell packaging contract test locks this branch.
- The candidate has UI Automation, bilingual keyboard, 100,000-entry browse/filter/cancel, and memory-baseline evidence; these do not constitute Narrator or formal accessibility certification.

## Remaining work

- [#14](https://github.com/ax2/zifile/issues/14): complete keyboard traversal, Narrator, Accessibility Insights, high contrast, Chinese IME, per-monitor DPI, and default-UI selection.
- [#15](https://github.com/ax2/zifile/issues/15)–[#18](https://github.com/ax2/zifile/issues/18): formal bilingual screenshots, WACK, WinGet community acceptance, and Microsoft Store certification.
- Real signing, Partner Center identity, physical ARM64, and the real foreground multi-operation queue remain incomplete.

## Release result

The RC engineering chain and public Alpha prerelease exist; the project is still in Release Candidate preparation, not a formal Store/WinGet release.
