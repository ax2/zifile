---
title: Roadmap
description: ZiFile's staged path from foundation validation to 1.0.
---

The repository-root [`ROADMAP.md`](https://github.com/ax2/zifile/blob/main/ROADMAP.md) is the single source of truth. This page is a reader-oriented summary.

| Stage | Goal | Key delivery |
| --- | --- | --- |
| Stage 0 | Validate the foundation | Rust workspace, Iced, CI, Starlight, ADRs, and bounded 100,000-entry list validation |
| Stage 1 | Alpha | ZIP/7z/TAR families, beta fixed-MSZIP Windows CAB creation, beta read-only RAR 1.3–7, safe extraction, bilingual UI, search and paging, progress, cancellation, all-format parser fuzzing, and 7-Zip/RAR/CAB reference corpora are implemented; the real foreground queue round remains a release gate |
| Stage 2 | Beta | Associations, taskbar progress, App Execution Alias, isolated Worker, dual-architecture packages, 100,000-entry browse/cancel baselines, and create/extract-to-matching-folder Explorer commands are implemented; trusted signed install, upgrade, and real Explorer activation remain |
| Stage 3 | RC | The Dioxus/WebView2 semantic candidate covers the main Worker flows, CSP, core shortcuts, labelled main-region focus after page changes, bilingual navigation/create-form keyboard regression, matching About/runtime-version pages, dual-architecture candidate packages, the complete high-DPI MSIX icon matrix, a reviewed 16/24/32/48/256 multi-resolution Win32 icon, bilingual docs, machine-validated Store copy, atomic screenshot import, WACK readiness, and a protected PFX-free cloud-signing/post-signing audit path; foreground focus, Narrator, Accessibility Insights, physical ARM64, real signing, formal screenshots, WinGet, and Store gates remain |
| Stage 4 (active) | 1.0 | Freeze APIs, finish documentation, and publish through all three channels |

Every stage has a dedicated work log covering goals, discoveries, changes, verification, remaining issues, and release outcome.

GitHub Milestones mirror the authoritative roadmap: Stage 1 queue work is [#11](https://github.com/ax2/zifile/issues/11); Stage 2 trusted install/Explorer and ARM64 are [#12](https://github.com/ax2/zifile/issues/12)–[#13](https://github.com/ax2/zifile/issues/13); Stage 3 accessibility, formal screenshots, WACK, Store, and WinGet are [#14](https://github.com/ax2/zifile/issues/14)–[#18](https://github.com/ax2/zifile/issues/18); the cross-channel 1.0 gate is [#19](https://github.com/ax2/zifile/issues/19). Partner Center and signing remain [#8](https://github.com/ax2/zifile/issues/8)–[#9](https://github.com/ax2/zifile/issues/9).
