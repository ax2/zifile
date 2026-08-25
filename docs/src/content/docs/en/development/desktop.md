---
title: Desktop use and accessibility
description: Languages, themes, shortcuts, large archives, and open accessibility gates.
---

ZiFile's desktop is written in Rust. Compression, testing, and extraction run in background Workers with entry/byte progress and cancellation. File names, contents, and passwords are never uploaded.

## Settings and shortcuts

The first launch selects Simplified Chinese or English from the system locale. Language and light/dark theme can be changed at any time. Only those two preferences are stored in `%LOCALAPPDATA%\ZiFile\settings.conf`; passwords, paths, and recent files are not persisted.

| Shortcut | Action |
| --- | --- |
| `Ctrl+O` | Open an archive |
| `Ctrl+N` | Open the create page |
| `Ctrl+A` | Select every entry while the archive region is focused |
| `Escape` | Cancel the current cancellable operation |

Search is immediate and results are paged at 500 rows, keeping 100,000-entry archives bounded. Safety limits still apply during listing. Worker byte progress, or entry progress when bytes are unavailable, is mirrored to the Windows taskbar.

## Operation queue

Open, reload, test, extract, and create requests may be submitted while work is running. A 32-item in-memory FIFO executes snapshots in order. Clearing removes only waiting work; cancel affects only the current Worker and then advances the queue. Paths and passwords are released after clearing, completion, or exit and are never written to settings or logs.

Unit tests cover FIFO ordering, capacity, stale completions, clearing, and payload release. A real foreground multi-operation smoke run is still required before the roadmap queue item can close.

## Accessibility evidence and limits

The opt-in Dioxus/WebView2 candidate shares the Worker and supports the primary browse, test, selective-extract, create, progress, cancel, drop, and shortcut flows. Windows UI Automation has identified semantic controls; real bilingual keyboard flows, bounded 100,000-entry browsing, cancellation, x64 runnable/MSIX execution, and x64/ARM64 cloud packaging have passed.

These checks are not full certification. Archive/extract traversal, visible focus, Narrator, Accessibility Insights, physical high contrast, Chinese IME, per-monitor DPI, real cross-window drop, physical ARM64 execution, and WACK remain release gates. Build the candidate with:

```powershell
cargo build -p zifile-desktop --features accessible-ui --bin zifile-desktop-accessible
target\debug\zifile-desktop-accessible.exe sample.zip
```
