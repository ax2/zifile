---
title: Roadmap
description: ZiFile's staged path from foundation validation to 1.0.
---

The repository-root [`ROADMAP.md`](https://github.com/ax2/zifile/blob/main/ROADMAP.md) is the single source of truth. This page is a reader-oriented summary.

| Stage | Goal | Key delivery |
| --- | --- | --- |
| Stage 0 | Validate the foundation | Rust workspace, Iced, CI, Starlight, ADRs, and bounded 100,000-entry list validation |
| Stage 1 (active) | Alpha | ZIP/7z/TAR families, safe extraction, bilingual UI, search and paging, progress, cancellation, all-format parser fuzzing, and a nine-case official 7-Zip corpus are implemented; malformed and bomb corpora continue to expand |
| Stage 2 | Beta | Associations, taskbar progress, App Execution Alias, isolated Worker, dual-architecture packages, and 100,000-entry browse/cancel baselines are implemented; signed install/upgrade and Explorer commands remain |
| Stage 3 | RC | The Dioxus/WebView2 semantic candidate covers the main Worker flows, CSP, core shortcuts, bilingual navigation/create-form keyboard regression, dual-architecture candidate packages, and 18 equivalent bilingual documentation pairs; archive forms, visible focus, Narrator, Accessibility Insights, physical ARM64, WinGet, Store, and supply-chain gates remain |
| Stage 4 | 1.0 | Freeze APIs, finish documentation, and publish through all three channels |

Every stage has a dedicated work log covering goals, discoveries, changes, verification, remaining issues, and release outcome.
