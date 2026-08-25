---
title: "ADR-0005: Accessible desktop UI migration"
description: The Iced accessibility gap, licensing constraints, and candidate migration path.
---

## Status

Accepted for validation on 2026-08-24. Iced remains the release baseline until the candidate proves functional parity, licensing, and both architectures.

## Background and candidates

Store quality requires keyboard access, Narrator/UI Automation, high contrast, DPI, and IME support. Iced 0.14 does not currently expose a complete semantic accessibility tree. Labels drawn on canvas controls are insufficient for Windows assistive technology.

Slint exposes operating-system accessibility properties, but its GPLv3, royalty-free, or commercial runtime choices complicate a purely permissive distribution. Dioxus Desktop uses Rust RSX with the system WebView2 on Windows, allowing semantic HTML to use the browser accessibility implementation under MIT/Apache-2.0 licensing. A custom AccessKit adapter for Iced would amount to maintaining toolkit-level infrastructure and is not the product path.

## Direction and evidence

The opt-in `zifile-desktop-accessible` candidate uses Dioxus Desktop/WebView2 while preserving Rust state, Worker IPC, taskbar integration, settings, and core safety limits. It implements home, archive browsing/filtering/paging/selection, integrity testing, extraction, creation, progress, cancellation, command-line open, native drop, and core shortcuts. CSP limits resources to the local UI, Dioxus protocol, and loopback WebSocket.

Windows UI Automation has identified landmarks, headings, tables, checkboxes, combo boxes, sliders, password fields, and live status. Archive selection now exposes distinct select-all/clear-all actions, an atomic live count referenced by the archive region and extract button, and item/count status after each change. The create-source list also exposes a live count and path-specific removal controls that remove by identity rather than a stale index; bilingual Rust tests cover these semantic states. The global announcer reserves atomic alert/assertive semantics and visible error emphasis for Worker/queue failures. Status, queue, and progress are separate regions so 100 ms progress refreshes do not mutate the atomic live region; the progress element exposes percentage, bytes, and entries on demand, while Cancel and Clear queue reference their relevant summaries. Real archive, 100,000-entry, cancellation, bilingual keyboard, x64 runnable-directory/MSIX, and cloud x64/ARM64 package checks have passed. This is not yet Narrator certification or proof of visible focus, physical ARM64 execution, real high-contrast visuals, Chinese IME, per-monitor DPI, or cross-window drag-and-drop. The candidate replaces Iced only after those gates pass.
