---
title: Roadmap
description: ZiFile's staged path from foundation validation to 1.0.
---

The repository-root [`ROADMAP.md`](https://github.com/ax2/zifile/blob/main/ROADMAP.md) is the single source of truth. This page is a reader-oriented summary.

| Stage | Goal | Key delivery |
| --- | --- | --- |
| Stage 0 | Validate the foundation | Rust workspace, Iced, CI, Starlight, ADRs, and bounded 100,000-entry list validation |
| Stage 1 (active) | Alpha | ZIP/7z/TAR families and beta read-only RAR 1.3–7, safe extraction, bilingual UI, search and paging, progress, cancellation, all-format parser fuzzing, and cloud 7-Zip/RAR reference corpora are implemented; malformed/bomb campaigns continue |
| Stage 2 | Beta | Associations, taskbar progress, App Execution Alias, isolated Worker, dual-architecture packages, and 100,000-entry browse/cancel baselines are implemented; signed install/upgrade and Explorer commands remain |
| Stage 3 | RC | The Dioxus/WebView2 semantic candidate covers the main Worker flows, CSP, core shortcuts, bilingual navigation/create-form keyboard regression, dual-architecture candidate packages, 20 equivalent bilingual documentation pairs, machine-validated Store copy, atomic screenshot import, and WACK readiness; archive forms, visible focus, Narrator, Accessibility Insights, physical ARM64, formal screenshots, WinGet, Store, and supply-chain gates remain |
| Stage 4 | 1.0 | Freeze APIs, finish documentation, and publish through all three channels |

Every stage has a dedicated work log covering goals, discoveries, changes, verification, remaining issues, and release outcome.

GitHub Milestones mirror the authoritative roadmap: Stage 1 queue work is [#11](https://github.com/ax2/zifile/issues/11); Stage 2 trusted install/Explorer and ARM64 are [#12](https://github.com/ax2/zifile/issues/12)–[#13](https://github.com/ax2/zifile/issues/13); Stage 3 accessibility, formal screenshots, WACK, Store, and WinGet are [#14](https://github.com/ax2/zifile/issues/14)–[#18](https://github.com/ax2/zifile/issues/18); the cross-channel 1.0 gate is [#19](https://github.com/ax2/zifile/issues/19). Partner Center and signing remain [#8](https://github.com/ax2/zifile/issues/8)–[#9](https://github.com/ax2/zifile/issues/9).
