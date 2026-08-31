---
title: Testing strategy
description: Unit, property, interoperability, security, performance, fuzz, and smoke gates.
---

## Pull-request gates

- Rust format, strict all-target/all-feature Clippy, and locked all-target tests.
- License, provenance, and advisory policy.
- Starlight type checking and static build.
- Criterion and libFuzzer target compilation.
- Real Worker and packaging-policy smoke tests.
- Windows ZIP, tar.gz, tar.lzma, and 7z bidirectional interoperability.
- Official 7-Zip codec, filter, solid-mode, and AES interoperability with JSON evidence.
- Pinned RAR 1.3/1.5/3/5/7, PPMd, filter, encrypted-header and unsafe-link corpus cross-checked against 7-Zip with JSON evidence.

The documentation locale check requires one-to-one Chinese and English pages
and also rejects Markdown files whose body is empty after front matter is
removed, preventing a present-but-blank page from passing CI.

## Layers

Unit and property tests cover detection, paths, limits, conflict policy, randomized trees, and boundaries. Security corpora cover traversal, bombs, links, collisions, corruption, and truncation. Interoperability compares ZiFile in both directions with Windows reference tools. Performance tracks throughput, ratio, peak memory, startup, and large lists. Smoke tests cover CLI, desktop, Worker IPC/cancellation, packaging, installation, associations, and uninstall as their environments become available.

Windows CI uses PowerShell `Compress-Archive`/`Expand-Archive` and the system `tar.exe` for bidirectional ZIP, tar.gz, tar.lzma, and 7z interoperability; `tar.exe --lzma` covers the LZMA-alone composition. Reference-created archives are tested and extracted by ZiFile, while ZiFile-created archives are extracted and compared file by file by the reference tool. The ZIP, tar.gz, and 7z cases include Unicode paths; the reverse tar.lzma case also checks ASCII file content so a Windows reference-tool code-page difference is not misreported as a ZiFile result. Each run uploads structured evidence containing no user data.

A permanent negative matrix constructs minimally signed or extension-hinted truncated/corrupt inputs for all 17 supported format classes. Both List and integrity testing must reject each input as an ordinary error without panicking. This complements rather than replaces continuous libFuzzer campaigns, the historical 7z crash fixtures, and real third-party corpora.

The Worker smoke streams a real list request and requires a complete final progress snapshot before metadata, a Unicode entry, and exactly one terminal event. It then cancels a 32 MiB random 7z creation and requires timely exit with no target or temporary residue. Queue unit tests cover strict FIFO, 32-item capacity, stale completion IDs, clearing, and immediate sensitive-payload release.

Foundation smoke also runs a real Windows CLI round-trip matrix for the major creatable formats: TAR+Zstandard, TAR+XZ, TAR+Bzip2, gzip, Zstandard, XZ, LZMA, Bzip2, LZ4, and Brotli. Every case creates an archive, integrity-tests it, extracts it, and asserts Unicode-file or single-stream output content. ZIP, TAR+gzip, TAR+LZMA, and AES 7z have separate scenarios in the same smoke flow, so the matrix verifies behavior rather than only a capability row or file existence.

Listing and integrity testing use backward-compatible `ListOptions` and `TestOptions` to provide common progress and cooperative cancellation for ZIP, 7z, RAR, CAB, TAR compositions, and all seven compression streams. Listing advances as entries are scanned; single streams also report actually decoded bytes. Both UIs use explicit scanning copy while a total is unknowable, then receive a consistent final total. Integrity testing and extraction reuse the same progress channel for their internal preflight, but reset scanned counters when the main phase begins so a streamed file cannot appear 100% complete before it is actually written. The Worker emits bounded updates every 100 ms and a final snapshot after the operation returns, so small archives do not expose only an initial 0% value. A pre-cancel regression requires `Cancelled` before parsing begins, and every format round trip checks the final progress invariants.

The CAB filename-collection helper also uses the caller's entry limit and checks cooperative cancellation, keeping its temporary name set within the shared resource boundary.

Integrity testing computes a stable lowercase hexadecimal SHA-256 while decoding
regular-file contents; directories and metadata-only listing do not receive a
checksum. The value travels through the existing `ArchiveEntryInfo`/Worker
event, while legacy events missing the optional field still deserialize. ZIP,
7z, TAR families, stream formats, RAR, and CAB regressions verify that checksum
results and completed progress agree; the CLI and both desktop tables expose the
same value. The table action is also covered for bilingual labels, native Iced
clipboard wiring, and the accessible UI's Clipboard API failure fallback.

CLI password tests cover explicit opt-in, CRLF/LF removal, preservation of surrounding spaces, and rejection of missing or empty input. The foundation smoke requires help to expose only `--password-stdin`, then creates, tests, and extracts a real AES 7z through standard input without printing the fixed test password.

Both desktop UIs share one bilingual formatter for stable Worker errors. Unit tests cover cancellation, password-required, unknown-format, destination-conflict, and safety-limit messages, while requiring paths and backend diagnostic details to survive; unknown backend errors remain unchanged for troubleshooting.

Compression-level contract tests cover every encoder's valid range, core boundary clamping, and fixed-level formats. CLI tests require `zifile formats` to expose each format's range, `fixed`, or `none`, and reject both format-specific out-of-range values and explicit levels for fixed formats before creation so command-line input is neither silently clamped nor ignored. The foundation smoke also launches the real `zifile.exe`, checks key matrix rows, rejects ZIP level 10 and an explicit TAR level, then proves TAR creation succeeds when the level is omitted. A 7z integration regression creates level-0 and level-9 archives, reads the public LZMA2 coder properties, and requires them to differ. This proves from archive metadata that the UI/CLI setting affects the encoder instead of merely traversing the Worker protocol.

`tests/performance/operation-queue-foreground.ps1` creates a deterministic ZIP with 100,000 entries of 1 KiB each by default, submits three integrity tests through the real window, and verifies cancellation of the active operation, start of the next operation, clearing the waiting queue, and Worker reclamation; `-EntrySizeBytes` adjusts the fixture workload within a bounded range. To keep the queue window deterministic instead of dependent on disk speed, the script defaults to adding a 10-second startup window to each Worker operation through the test-only `ZIFILE_TEST_WORKER_DELAY_MS`; `-WorkerDelayMilliseconds` can adjust it from 0 to 10 seconds, and cancellation remains observed during the delay. The production default is disabled. A separate bounded three-second activation retry first requires the ZiFile native handle to be the actual Windows foreground window; if the session cannot grant foreground ownership, the test refuses to run rather than recording background UI Automation as foreground evidence. Text timeouts include at most 500 characters of the last visible Document text; button timeouts also include at most 32 button names and `IsEnabled` states, while `finally` still closes the process and removes the temporary fixture.

`tests/smoke/packaging-policy.ps1` dynamically parses the current thirty-three release, corpus, and repository-policy PowerShell scripts. It rejects missing/partial Partner Center identity, malformed Name/X.500 Publisher, missing cloud inputs, unsupported providers, development identities, unsigned OID publishers, invalid signed artifacts, and incomplete 1.0 readiness; it accepts valid input and fully evidenced 11/11 fixtures. It also requires post-signing audit, signed-only publishing, least privilege, signing timeout/concurrency, the rotation/emergency-stop/revocation runbook, and version, release-note, contributor, security, and readiness gates in CI. Every CI, documentation deployment, SBOM, and GitHub Release publication job has a workload-sized hard timeout, with job-scoped checks preventing an unrelated limit elsewhere in the same YAML from satisfying the gate. The dual-architecture reproducibility build is separately protected by a 120-minute hard timeout, stale-run cancellation, independent matrix conclusions, and retained failure evidence; real foreground ownership and bounded diagnostics in the queue smoke are also regression-checked. Policy smoke cannot replace a real account, cloud-HSM signature, or x64/ARM64 package-content audit.

The Explorer Shell policy requires `zifile-shell` to reuse the core format detection and capability registry and to require a real file for extraction. It rejects a reintroduced independent extension allowlist, preventing the core formats, file associations, and context-menu coverage from drifting apart. Shell state evaluation and both desktop create forms reject symbolic links, junctions, and other reparse-point sources, so the menu or Create button does not appear enabled only to fail later in the Worker. The shared creation preflight also covers sources that disappear after selection, so recoverable input errors are reported before the save dialog rather than only after Worker startup.

The official 7-Zip corpus gate uses `7z.exe` on the GitHub Windows Runner. Reference-created cases cover Copy, LZMA, LZMA2+BCJ, Deflate, BZip2, PPMd, and LZMA2+AES with encrypted headers. In the reverse direction, 7-Zip must test and extract both ordinary and AES archives created by ZiFile. Every case compares the complete relative file set and SHA-256 content hashes; the uploaded JSON evidence contains no password. CI `32836336921` passed all nine cases with 7-Zip 26.02; the evidence JSON SHA-256 is `06278BB8B96AB683A3C117BA5E30F1B4AB1CF89F1BBF01E72BAC0CC26B49DB14`.

The RAR gate downloads six fixtures from the pinned `rars` source commit `7d8f9386ef777a2415da34fe1db193d8471ff7d0`, verifies hard-coded SHA-256 values before use, and compares extraction trees byte for byte. It covers RAR 1.3, 1.54 multi-file, RAR 3 PPMd, RAR 5 compression and E8E9 filtering, plus a WinRAR 7.21 encrypted-header/Quick Open archive. Three pinned link/redirection archives must be rejected without output. CI `32853686537` passed all six valid and three rejection cases; the evidence JSON SHA-256 is `4C52D0240B911609C7DDB0CACB2E484F56C8F886E216347603B228261C4EE8EF`. Because current 7-Zip no longer reads RAR 1.3, that case is compared with the known-good extracted tree from the same pinned upstream commit; the other five valid archives remain cross-checked against 7-Zip 26.02.

The CAB interoperability gate uses Windows `makecab.exe` to create MSZIP and LZX cabinets, then requires ZiFile signature detection, listing, testing, and extraction to match the SHA-256 output from system `expand.exe`. Rust integration fixtures cover uncompressed CAB content. Quantum compression and multi-cabinet sets remain explicitly unsupported. CI uploads structured evidence containing no user data.

A CAB decode-stage negative regression preserves valid metadata while flipping the first compressed CFDATA byte. Listing must still expose the one entry, then integrity testing and selective extraction must fail with an empty destination. This proves corrupt payloads cannot cross the temporary-file commit boundary instead of treating malformed-header coverage as decoder coverage.

Modification-time tests set deterministic source times, create ZIP/7z/TAR-family archives, and require both listed metadata and extracted files/directories to retain the expected values. Separately authored RAR 5 and CAB fixtures cover read-only providers. Protocol tests deserialize a legacy `archive_entry` event without the optional field, while both desktop binaries share formatting tests that distinguish UTC from timestamp fields whose archive format stores no time zone.

Every CI compiles fuzz targets. Weekly bounded campaigns exercise path policy, format detection, and every supported parser, including CAB, for 180 seconds each. Two historical malformed 7z artifacts (292 and 173 bytes) are replayed at every parser campaign start. Their discoveries led to Rust 1.93.0 and bounded-metadata `sevenz-rust2` 0.22.0; targeted run `32813469578` replayed both, executed another 498,937 inputs in 181 seconds, peaked at 370 MiB RSS, and found no new crash.

The RAR verification benchmark uses a deterministic 8 MiB RAR 5 method-3 archive with low-frequency pseudorandom noise, retaining compression work without exceeding the default 1000:1 expansion guard. The initial local Windows x64 baseline measured 58.12–64.49 ms, or 124.06–137.65 MiB/s. This is a same-machine regression baseline, not a universal performance claim. The original highly periodic fixture was correctly rejected as exceeding the safety ratio and was not used to bypass that guard.

The 100,000-entry UI model constructs at most 500 visible rows. A real deterministic ZIP baseline validates Worker listing, search, paging, 50% scrolling, tree-wide memory sampling, and cancellation with Worker reclamation. Five cancellation runs completed at 930.78 ms median and 1088.73 ms p95 with zero Workers remaining. These are same-machine regression baselines, not universal performance promises.

`tests/performance/operation-queue-foreground.ps1` uses a 100,000-entry ZIP in a real foreground session to verify FIFO submission, active cancellation, next-operation progress, clearing pending work, and Worker reclamation. It supports both the default Iced desktop and the accessible Dioxus candidate, with bilingual UI text matching. `tests/performance/extraction-cancellation-foreground.ps1` starts the real `--extract-here` flow with a deterministic multi-entry ZIP, cancels extraction, waits for Worker reclamation, and verifies that every committed file has a complete entry size with zero partial output files. Both scripts require an available interactive desktop; missing semantic Document UIA or an unrun foreground session remains incomplete evidence.

The core test `active_cancellation_does_not_commit_a_partial_zip_output` cancels ZIP extraction after the first progress data chunk and requires `Cancelled` with no committed atomic target file. It passed five repeated local runs. This strengthens core-layer evidence but does not replace the real desktop queue acceptance run.

Creation path regressions reject a destination equal to or inside a source tree before creating its parent. The check resolves the nearest existing ancestor and compares the resulting paths case-insensitively on Windows, preventing a temporary output from being re-enumerated as input and leaving no empty directory behind for this recoverable error.

If the extraction destination already exists as a regular file, the core returns structured `DestinationExists` before creating a directory and preserves the original file contents. This is tested separately from entry-level conflict policies, so a non-directory destination is not exposed as an unstable raw I/O failure.

The extraction root and every existing output parent reject symbolic links, junctions, and reparse points. A regression confirms that the link target receives no file, preventing a pre-existing host link from bypassing the archive path policy.

When creating an archive, every temporary-file commit path now returns structured `DestinationExists` if the output file already exists and preserves the original contents. This is separate from extraction's explicit entry-conflict policy, avoiding platform-dependent replacement or raw rename errors.

The expansion-ratio limit uses overflow-free exact multiplication instead of integer division. For example, with a `1000:1` limit, 2,001 expanded bytes from 2 compressed bytes are rejected rather than truncated to an apparent ratio of `1000`.

The preferences regression replaces an existing configuration in a temporary directory, checks complete bilingual language/theme content, and confirms that the temporary file is removed after a successful commit. Windows uses the `MOVEFILE_WRITE_THROUGH` replacement path.

Single-stream listing applies the caller-provided expansion-ratio limit as well. A configured ratio of `0` strictly permits no decoded output; it is not silently widened to one compressed byte of output. A gzip regression covers this boundary so listing and declared-entry validation cannot use different safety policies.

TAR and its five compressed compositions accumulate declared entry sizes immediately after each header is parsed, then compare the total with expanded-size and ratio limits before skipping compressed payloads. Listing therefore does not decode an over-budget payload merely to reach the next header. The regression matrix covers TAR, TAR+gzip, TAR+Zstandard, TAR+XZ, TAR+LZMA, and TAR+Bzip2.

Both creation UIs now reject symbolic-link, junction, and reparse-point sources before opening the save dialog, matching the core link-rejection policy and presenting bilingual recovery guidance that identifies link-like sources and asks the user to choose the original file or folder.

Windows entry collision keys use Unicode lowercasing; a Windows-only ZIP regression uses `Ä.txt` and `ä.txt` to require rejection during listing, before two case variants can land on the same filesystem path.

CI also runs a Windows `performance` job that executes the `format_detection` and `archive_throughput` Criterion benchmarks with a fixed 10-sample configuration. It retains the text output, Criterion baseline, and HTML data as a 30-day artifact. Criterion's relative-regression hint still requires review against a same-machine history; the job's pass first proves that the benchmarks actually ran without a runtime failure.

`archive_throughput` now measures both ZIP Deflate and TAR + LZMA-alone. The latter uses an independent 1 MiB sample for creation and integrity testing, so the new composition has a repeatable throughput observation without making the benchmark workload unbounded.

`tests/smoke/contract-policy.ps1` locks the six public CLI commands, fifteen creation formats, all seventeen `formats` capability rows, both public contract documents, and runtime error code 1 versus syntax error code 2. It runs after the Foundation smoke in Windows CI; the final 1.0 freeze still belongs on the release commit.

Shared desktop tests also cover drop classification: archive content signatures take precedence over extensions so renamed archives still enter the browser. Renamed TAR+gzip, TAR+Zstandard, TAR+XZ, and TAR+Bzip2 files whose outer signatures are ambiguous use a bounded probe of at most 1 MiB of compressed input and a 512-byte decoded header to identify the inner TAR. Known extensions retain a compatibility fallback when probing fails, while ordinary files and archive-named directories are not treated as openable archives.
The Iced and Dioxus entry paths are also source-locked to asynchronous/background probing, preventing header reads from returning to the UI event thread.

File-dialog coverage locks the open-archive, extraction-folder, add-file/folder, and save-archive paths in both UIs away from direct waits on the UI event thread. Iced uses a controlled Tokio blocking task and Dioxus uses `AsyncFileDialog`. An active-dialog guard prevents repeated clicks from creating duplicate native windows, and both cancellation and completion release the guard.

Archive-browser regressions also require both UIs to show a bilingual empty state when the current folder is empty or a search has no matches, rather than rendering an ambiguous blank table; clearing search preserves the folder and returns to page one.

Open-failure regressions distinguish password/encryption diagnostics from corruption, unknown formats, and ordinary I/O errors: only a likely password case presents password input and an Unlock retry. Both UIs also reject stale Worker completions before applying results, so an old task cannot overwrite the current archive or busy state.

Password-visibility regressions cover default masking, archive/create field wiring in both UIs, and restoring masking whenever a password is released. The accessible candidate additionally locks three unique field IDs, native checkbox labels, dynamic `password`/`text` types, and matching `aria-controls`. This source and unit evidence does not replace a real screen-reader or foreground visual pass.

Encrypted creation also requires an exact confirmation in both UIs before the save dialog can open. A shared pure-function regression covers matching empty values, exact matches, case-sensitive mismatches, and either field being empty. The mismatch gate is always active, while inline feedback waits until the user starts interacting with the confirmation field. The accessible candidate exposes a separately labelled confirmation field, `aria-invalid`, an alert description, and one visibility checkbox scoped to both fields. Accepted create submissions clear both transient values and the interaction state; format changes that disable encryption do the same.

The encrypted-file-list password input supports `Enter` submission. The default UI reuses the existing reload path through Iced's input submission event. A pure helper regression for the accessible candidate requires an enabled, non-composing input with no Ctrl/Alt/Shift/Meta modifiers, permits lock-state modifiers, and prevents duplicate submission while busy.

Create-page regressions also require the default Iced and accessible Dioxus UIs to use the same bilingual status announcements for adding, removing, and clearing sources. The status includes the change and remaining total, so a list update cannot silently lose feedback for the user or assistive technology.

Archive-list interaction regressions require only directory names to use actionable navigation buttons. Regular file names remain text, while each row's checkbox owns selection for both files and directories. This avoids presenting regular files as disabled pseudo-buttons or implying a name action that does not exist, while retaining a clear directory-navigation target.

About-page link regressions require both UIs to expose actionable project, current-locale documentation, and current-locale privacy-policy destinations. The shared layer accepts only a compile-time `OfficialLink` enum: all five concrete destinations must use HTTPS and remain under the project GitHub or GitHub Pages paths, so arbitrary user strings never reach the system protocol launcher. Windows uses `ShellExecuteW` to invoke the default browser, and both success and failure update the application status region.

Save-path regressions require both desktop variants to add the selected format's canonical suffix when the user omits an extension, while preserving an explicitly entered alternative suffix. Compound formats such as TAR + gzip must produce the complete `.tar.gz`, not only `.gz`.

Sorting regressions cover folder-first order, ascending/descending direction, missing modified times last, first-page reset, and the 500-row cap. The Criterion suite also sorts all 100,000 entries by name descending before collecting one page; the initial Windows x64 measurement was 13.96–15.32 ms (6.53–7.17 million entries/s). Header helper tests require the visible arrow and Dioxus `aria-sort` value to agree.

Folder-browser regressions cover explicit and implicit directories, direct children at root and nested levels, navigable breadcrumbs, archive-wide search, and resetting search/page state when entering a folder. The 100,000-entry fixture synthesizes one root folder and still collects at most 500 rows after entering it in either sort direction. On the initial local Windows x64 baseline, scanning 100,000 paths and synthesizing the root took 18.60–19.44 ms; entering the folder, sorting by name descending, and collecting 500 rows took 38.04–38.74 ms.

Folder-selection regressions cover one-pass all/partial/none counts for each direct child folder, adding or removing only the target descendants, disabled empty-folder semantics, and directory-row precedence when a file conflicts with an implicit directory. Dioxus source gates require the mixed state, bilingual selected/total labels, and deterministic row keys containing the page-local index. On the local Windows x64 baseline, one root aggregation across 100,000 entries with half selected took 30.97–32.75 ms.

`tests/smoke/store-listing.ps1` verifies that the Simplified Chinese and English Store JSON satisfies Partner Center limits for descriptions, short descriptions, features, keywords, system requirements, licensing, and HTTPS URLs. It also requires each readable listing page to contain every authoritative JSON description paragraph and feature verbatim. Negative fixtures prove that an oversized feature, excess keywords, and a URL inside the description are rejected. This gate covers copy, not screenshots, age ratings, official identity, or certification.

The same smoke test exercises atomic screenshot import: it generates eight valid PNGs, requires complete capture metadata, imports from an independent directory, and reruns the formal manifest validator. Missing metadata, undersized images, duplicate content, and attempts to overwrite existing assets all fail. Temporary images are removed and never enter the formal asset directory.

`tests/helpers/msix-repair` is a C# test-only console helper; the product remains Rust-first. CI compiles it against a locked Windows App SDK 1.8 dependency, then a PowerShell supervisor that does not load the App SDK launches the non-mutating `--probe`. Even if App SDK initialization blocks before the helper entry point, the supervisor terminates the process directly after 15 seconds; the workflow adds a two-minute outer bound. A Runner that does not return records an incomplete/unsupported probe instead of hanging or claiming Repair passed, and a one-second blocking fixture continuously proves this hard-timeout path. When Repair is supported, the trusted lifecycle writes a random package LocalState sentinel, requires `RepairPackageAsync` to preserve it, then requires `Reset-AppxPackage` to remove it. Unsupported systems record `unsupported` explicitly.

`tests/smoke/wack-readiness.ps1` uses an unsigned development-package fixture to prove readiness reports a missing WACK tool, invalid signatures, mismatched Partner Center Identity/Publisher/Publisher Display Name values, an unsigned publisher, wrong minimum OS, and package/audit hash mismatch. It also proves `-RequireReady` persists structured failure evidence. This smoke does not run WACK or replace a formal signed-candidate certification report.

Foreground keyboard automation checks internal WebView2 focus, bilingual forward/reverse navigation, disabled-control skipping, 7z selection, level adjustment, password clearing, and source buttons. Its default flow also creates a two-entry ZIP and checks archive listing, integrity testing, scoped Ctrl+A in the archive password field, Reload, committed search and scoped Ctrl+A, archive select-all/clear, conflict policy, disabled single-page pagination, and extract-button state; `-SkipArchiveWorkflow` is only an isolation mode for the legacy create-form regression. It verifies the exact ZiFile foreground window before every key and never records the password. An independent Windows x64 foreground UIA run now passes this workflow; the raw JSON is archived with the project records. This evidence does not replace Narrator, IME, high-contrast, DPI, or formal assistive-technology certification.

The keyboard smoke keeps its assertions strict while tolerating a bounded focus-delivery race: a forward or reverse navigation key may be resent only while focus remains on the current control. The final focus and ValuePattern values must still match exactly.

The accessible candidate exposes every handled shortcut—`Ctrl+O`, `Ctrl+N`, `F1`, `Escape`, and archive-scoped `Ctrl+A`—on the corresponding semantic control. A source regression keeps handler behavior and `aria-keyshortcuts` metadata aligned; this wiring evidence does not replace a real screen-reader announcement check.

The root keyboard handler prevents the default Escape action only when an active cancellation token exists. While idle it leaves Escape unhandled so native controls retain their close or exit behavior; the cancellation function also remains a no-op instead of publishing a false cancelling status when no token exists.

Shortcut matching ignores lock states such as Caps Lock and Num Lock but requires every other modifier to match the published contract exactly. Consequently, `Ctrl+Shift+N`, `Alt+F1`, and `Ctrl+Shift+A` are not downgraded to `Ctrl+N`, `F1`, or `Ctrl+A`. Regression coverage includes positive combinations, lock-state compatibility, and Shift/Alt negative cases.

The default Iced desktop follows the same exact functional-modifier rule for `Ctrl+O`, `Ctrl+N`, archive-scoped `Ctrl+A`, `F1`, and `Escape`. Its modifier type contains only Shift, Control, Alt, and Logo, so unmodified commands require `NONE` and Control commands require exactly `CTRL`; a pure event-mapping regression covers positive and extra-modifier cases.

When the accessible candidate changes between Home, Archive, Create, and About, it moves focus to the stable main region and labels that region with the active page heading. The effect is guarded by the page identity, so progress polling, queue updates, filtering, selection, and other same-page renders do not steal focus. Source and unit regressions lock the page-to-heading mapping and focus trigger; real Narrator and foreground keyboard validation remain required.

The accessible candidate uses a two-tone focus ring in its normal dark and light themes: the outer tone follows the theme while an opposing inner tone keeps focus distinguishable on both content surfaces and cyan active controls. Windows forced-colors mode uses `Highlight` and `Canvas`. A Rust source regression locks all three branches and the two-layer wiring; a real foreground keyboard and high-contrast pass is still required for visual evidence.

Reproducibility separately performs clean x64/ARM64 double builds. Schema-v2 evidence traced the former 4/5 result to `build-a`/`build-b` target paths embedded by generated `glutin_wgl_sys` code in the default Iced executable. The script remaps both isolated roots to one virtual path; run `32826187552` then proved 5/5 and `reproducible=true` on both architectures.
