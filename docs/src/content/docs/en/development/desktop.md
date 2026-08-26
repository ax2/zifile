---
title: Desktop use and accessibility
description: Languages, themes, shortcuts, large archives, and open accessibility gates.
---

ZiFile's desktop is written in Rust. Compression, testing, and extraction run in background Workers with entry/byte progress and cancellation. File names, contents, and passwords are never uploaded.

## CLI password input

The CLI does not accept `--password <value>`, which would expose a secret through process arguments and ordinary shell history. Encrypted `list`, `test`, `extract`, and `create` operations use `--password-stdin` to read one non-empty line from standard input. Only line endings are removed; leading and trailing spaces remain part of the password.

```powershell
$password | zifile test archive.7z --password-stdin
$password | zifile extract archive.7z output --password-stdin
```

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

The Windows 11 Explorer extension is a pure-Rust COM DLL with two modern commands. “Create archive with ZiFile” sends up to 256 selected sources to the create page. “Extract to matching folder with ZiFile” is shown only for one supported archive and launches the visible desktop with `--extract-here`. After signature-first listing succeeds, the desktop selects every regular file and extracts to a sibling folder matching the archive stem with rename-on-conflict behavior. Progress, cancellation, limits, and password retry remain in the desktop and isolated Worker; the DLL never parses archives or handles passwords. Real Explorer activation still requires a trusted installed package.

## Accessibility evidence and limits

The opt-in Dioxus/WebView2 candidate shares the Worker and supports the primary browse, test, selective-extract, create, progress, cancel, drop, and shortcut flows. Windows UI Automation has identified semantic controls; real bilingual keyboard flows, bounded 100,000-entry browsing, cancellation, x64 runnable/MSIX execution, and x64/ARM64 cloud packaging have passed.

The archive selection control now exposes an actionable “Select all archive files” or “Clear all archive selections” name and an atomic live “N of total” summary. The archive region and selective-extract button reference that summary with `aria-describedby`; individual selection changes report the path and current count through the global status. Pure Rust candidate tests cover bilingual actions, summaries, singular/plural status, and selection changes. This proves semantic wiring and state copy, not a real Narrator traversal.

Archive search now keeps a visible matching/total summary and connects the search input and results table through `aria-describedby` and `aria-controls`. A zero-result filter shows explicit bilingual empty copy and removes the meaningless “Page 1 / 1” pagination. Because filtering happens on every keystroke, the summary is deliberately `aria-live=off`; Enter announces the current result once through the global polite status. A dedicated Clear search action resets the query and page and announces the full count. Rust tests cover bilingual singular/plural, filtered, and empty-result copy.

The global announcer distinguishes information from failure. Queue, cancellation, and selection updates remain `status`/polite; Worker failures, a full queue, unexpected Worker output, and internal queue errors use atomic `alert`/assertive semantics plus visible normal- and forced-color emphasis. Status copy, queue count, and the progress element are now separate semantic regions, so a progress value updated every 100 ms is no longer inside the atomic live region. The progress element exposes percentage, processed/total bytes, and entry counts through `aria-valuetext`; Cancel references the current operation and exposes its Escape shortcut, while Clear queue references a singular/plural queue summary. Unit tests lock the bilingual progress/queue copy and “interrupt only for errors” contract. This code-level evidence still requires a real Narrator pass.

The create-source list now has an atomic live count. Every Remove button includes its full source path in its accessible name instead of exposing a set of indistinguishable controls. File/folder add, drop, remove, and clear actions announce the change and resulting count; removal matches the path rather than a potentially stale list index. Rust tests cover bilingual path/count copy and English singular/plural behavior.

Create-input requirements now come from the shared core capability model. ZIP, 7z, and TAR compositions accept files and folders, while the six single-stream formats require exactly one existing file. Both the Iced baseline and Dioxus candidate disable Add folder for stream formats, disable creation for an invalid selection, and show bilingual recovery guidance before opening a destination dialog. A defensive preflight also rejects invalid submissions that bypass the rendered control state.

These checks are not full certification. Complete real keyboard/Narrator archive and extract traversal, visible focus, Narrator, Accessibility Insights, physical high contrast, Chinese IME, per-monitor DPI, real cross-window drop, physical ARM64 execution, and WACK remain release gates. Build the candidate with:

```powershell
cargo build -p zifile-desktop --features accessible-ui --bin zifile-desktop-accessible
target\debug\zifile-desktop-accessible.exe sample.zip
```
