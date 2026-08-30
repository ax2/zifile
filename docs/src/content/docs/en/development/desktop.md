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

Preferences are written to a temporary file, flushed and synced, then atomically replaced on Windows; save failures are surfaced in both UIs instead of silently leaving a partial settings file.

| Shortcut | Action |
| --- | --- |
| `Ctrl+O` | Open an archive |
| `Ctrl+N` | Open the create page |
| `Ctrl+A` | Select every entry while the archive region is focused |
| `Escape` | Cancel the current cancellable operation |

Search is immediate and results are paged at 500 rows, keeping 100,000-entry archives bounded. Safety limits still apply during listing. Worker byte progress, or entry progress when bytes are unavailable, is mirrored to the Windows taskbar.

## Integrity-test checksums

`Test archive` decodes every regular file under the same safety limits and
records a lowercase hexadecimal SHA-256. After a successful test, both desktop
UIs replace the current metadata with the tested result and show the values in
the `SHA-256` column. Each value has an explicit `Copy checksum` action as well
as remaining directly reviewable. Directories remain `—`; a normal listing does
not decode file contents. The CLI prints the same result as a `SHA256\tPATH`
table.

After opening an archive, `Show in File Explorer` in the header opens its
containing folder and selects the current file. This is a local action; a
launch failure is surfaced as an error status, and the path is not written to
settings or logs.

## Operation queue

Open, reload, test, extract, and create requests may be submitted while work is running. A 32-item in-memory FIFO executes snapshots in order. Clearing removes only waiting work; cancel affects only the current Worker and then advances the queue. Both desktop UIs clear the create-form password as soon as a create request is accepted for execution or queuing, while retaining the input when a full queue rejects the request for retry. Request snapshots release paths and passwords after clearing, completion, or exit and never write them to settings or logs.

After successful creation or extraction, the status bar provides a bilingual `Show output` action that selects the generated archive or extraction directory in File Explorer. Starting new work clears the old output action; failures, cancellation, and Worker protocol result-type mismatches never leave a clickable success path, so status text and follow-up action cannot refer to different jobs.

While integrity testing, extraction, or creation is running, large archives use
a lightweight summary instead of rebuilding the entry table on every progress
refresh. Cancel, queue submission, opening another archive, and File Explorer
reveal remain available; the table returns when the operation finishes.

Unit tests cover FIFO ordering, capacity, stale completions, clearing, and payload release. A real foreground multi-operation smoke run is still required before the roadmap queue item can close.

The Windows 11 Explorer extension is a pure-Rust COM DLL with two modern commands registered for selected files, selected folders, and folder backgrounds. “Create archive with ZiFile” sends up to 256 selected sources to the create page; on a folder background it sends the current folder as the source. “Extract to matching folder with ZiFile” is shown only for one supported archive and launches the visible desktop with `--extract-here`. The DLL keeps Explorer's menu-building path bounded: when Explorer does not permit slow state evaluation it returns `E_PENDING`, and it resolves filesystem paths only for the allowed state pass or after invocation. After signature-first listing succeeds, the desktop selects every regular file and extracts to a sibling folder matching the archive stem with rename-on-conflict behavior. Progress, cancellation, limits, and password retry remain in the desktop and isolated Worker; the DLL never parses archives or handles passwords. Real Explorer activation still requires a trusted installed package.

When opening fails, the desktop distinguishes likely password-related archive errors from corruption, unknown formats, and ordinary I/O failures. Only the former presents password input and an Unlock retry, avoiding a misleading encryption diagnosis for every failure. Asynchronous Worker results also verify the active operation id before mutation, so an old task cannot overwrite the current UI state.

When a replacement archive is opened while another operation is active, the
default Iced UI keeps the currently visible archive until the replacement list
operation actually starts. A queued request or a request rejected because the
queue is full cannot clear the current archive or leave a misleading empty
state.

While a Worker is opening an archive, the empty state now says that the archive is opening instead of showing a failure message. The retry prompt appears only after a non-password failure, while a password-related failure shows the unlock form. The loading, failure, password, and idle states are locked by one shared bilingual formatter and unit tests used by both UIs.
When Explorer permits a slow state query, the extract command reuses the core `detect_format` and capability registry instead of maintaining a second extension allowlist. A valid archive can therefore receive the command even after being renamed, while an invalid file cannot receive it merely by claiming a `.zip` suffix. It also requires the Shell item to be a real file, so an ordinary folder named `backup.zip` does not receive a misleading extraction action.
The shell DLL also counts live COM objects and `LockServer` calls; `DllCanUnloadNow` succeeds only when both counts are zero, so Explorer can reclaim the extension without unloading it while a caller still holds a factory or command.
When Explorer supplies an empty `IShellItemArray` for a folder-background menu, the create command uses `IObjectWithSite` to query the current `IShellBrowser`/`IFolderView` and opens the create page with that folder as its single source; if the site is unavailable it disables or rejects the invocation instead of guessing a path.
The Shell create command also deduplicates Explorer sources by Windows path identity before launching the desktop, rejects disappeared paths, symbolic links, and junction/reparse-point sources, and applies the same command-line validation to file selections and folder-background invocations; the default Iced and accessible Dioxus create forms perform the same link-like preflight before opening the save dialog, an unusually deep current folder fails before the desktop starts, and extract invocation still requires exactly one Explorer item.
This follows Microsoft's [Windows 11 packaged desktop Explorer-command guidance](https://learn.microsoft.com/windows/apps/desktop/modernize/integrate-packaged-app-with-file-explorer): the manifest uses `*`, `Directory`, and `Directory\Background` targets, and expensive work is deferred until `Invoke`.

## Accessibility evidence and limits

The opt-in Dioxus/WebView2 candidate shares the Worker and supports the primary browse, test, selective-extract, create, progress, cancel, drop, and shortcut flows. Windows UI Automation has identified semantic controls; real bilingual keyboard flows, bounded 100,000-entry browsing, cancellation, x64 runnable/MSIX execution, and x64/ARM64 cloud packaging have passed.

The archive selection control now exposes an actionable “Select all archive files” or “Clear all archive selections” name and an atomic live “N of total” summary. The archive region and selective-extract button reference that summary with `aria-describedby`; individual selection changes report the path and current count through the global status. Pure Rust candidate tests cover bilingual actions, summaries, singular/plural status, and selection changes. This proves semantic wiring and state copy, not a real Narrator traversal.

Archive search now keeps a visible matching/total summary and connects the search input and results table through `aria-describedby` and `aria-controls`. A zero-result filter shows explicit bilingual empty copy and removes the meaningless “Page 1 / 1” pagination. Because filtering happens on every keystroke, the summary is deliberately `aria-live=off`; Enter announces the current result once through the global polite status. A dedicated Clear search action resets the query and page and announces the full count. Rust tests cover bilingual singular/plural, filtered, and empty-result copy.

The default Iced search bar now shows the same match summary. When there are zero results, its page position uses a placeholder instead of inventing “Page 1 / 1”, while the list still shows explicit empty-result copy. A shared bilingual formatter prevents pluralization and zero-result behavior from drifting between the two UIs.

The global announcer distinguishes information from failure. Queue, cancellation, and selection updates remain `status`/polite; Worker failures, a full queue, unexpected Worker output, and internal queue errors use atomic `alert`/assertive semantics plus visible normal- and forced-color emphasis. Status copy, queue count, and the progress element are now separate semantic regions, so a progress value updated every 100 ms is no longer inside the atomic live region. The progress element exposes percentage, processed/total bytes, and entry counts through `aria-valuetext`; Cancel references the current operation and exposes its Escape shortcut, while Clear queue references a singular/plural queue summary. Unit tests lock the bilingual progress/queue copy and “interrupt only for errors” contract. This code-level evidence still requires a real Narrator pass.

The default Iced UI now keeps the same `Informational`/`Error` status semantics: success, progress, queueing, cancellation, and source-selection updates use the normal footer style; Worker failures, create preflight failures, a full queue, preference-save failures, and internal queue errors use the danger theme. This makes failures visible without relying on users to read every status message. The mapping is covered by desktop tests and still needs real checks across Windows themes, forced-colors mode, and DPI settings.

The create-source list now has an atomic live count. Every Remove button includes its full source path in its accessible name instead of exposing a set of indistinguishable controls. File/folder add, drop, remove, and clear actions announce the change and resulting count; removal matches the path rather than a potentially stale list index. Rust tests cover bilingual path/count copy and English singular/plural behavior.

Create-input requirements now come from the shared core capability model. ZIP, 7z, and TAR compositions accept files and folders, while the seven single-stream formats require exactly one existing file. Both the Iced baseline and Dioxus candidate announce the selected format's source requirements, use a single-file picker for stream formats, reject Add folder, disable creation for an invalid selection, and show bilingual recovery guidance before opening a destination dialog. A defensive preflight also rejects invalid submissions that bypass the rendered control state and reports the failure with error semantics.

Drag-and-drop classification is shared by both desktops: it first reads a small header and uses content signatures, then falls back to a known extension hint when a format has no universal magic or probing fails. A valid ZIP/7z archive renamed to `.bin` can therefore still enter the browser, while ordinary files remain creation sources. Iced uses `Task::perform` and Dioxus uses `spawn_blocking` for the probe, so slow or remote files do not block the window; the Worker performs the definitive list operation.

Opening an archive, choosing an extraction directory, adding files/folders, and saving a new archive all use off-event-thread dialog paths. Iced places the native synchronous dialog in a controlled Tokio blocking task, while Dioxus uses `rfd::AsyncFileDialog`. Both UIs guard active dialogs until cancellation or completion, preventing network-directory stalls and duplicate native windows after repeated clicks.

Creation sources use the same shared path-identity deduplication function. On Windows, casing and slash-direction differences do not add the same file twice, and the Shell rejects disappeared paths, symbolic links, junctions, and reparse points before launch, so virtual items or stale sources cannot appear usable before the Worker reports an error.

The create-format menu order now comes from the core `ArchiveFormat::CREATABLE` registry. The Iced baseline and Dioxus candidate no longer maintain separate format arrays, so a new creatable provider receives the same stable ordering in both UIs. The contract smoke locks the current fifteen creation formats and prevents the menus from silently diverging from the public capability matrix.

When saving a new archive, both UIs add the selected format's canonical extension if the name entered in the native save dialog has no extension (for example, `backup` becomes `backup.zip` or `backup.tar.gz`). An explicit user-entered extension is preserved rather than silently replaced.

These checks are not full certification. Complete real keyboard/Narrator archive and extract traversal, visible focus, Narrator, Accessibility Insights, physical high contrast, Chinese IME, per-monitor DPI, real cross-window drop, physical ARM64 execution, and WACK remain release gates. Build the candidate with:

```powershell
cargo build -p zifile-desktop --features accessible-ui --bin zifile-desktop-accessible
target\debug\zifile-desktop-accessible.exe sample.zip
```
