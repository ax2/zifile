# Changelog

All notable changes to ZiFile are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Require an exact password confirmation before creating encrypted ZIP or 7z
  archives in both desktop UIs. Mismatches are shown inline, block the save
  dialog, and both transient values are released after an accepted request.

### Changed

- Render regular archive names as text while keeping directory names as explicit
  navigation buttons, so the file table no longer presents non-actions as
  disabled controls.

## [0.1.7] - 2026-08-31

### Added

- Add a bilingual recent-archives section to both desktop home pages. The eight
  most recently opened archives are persisted only after successful listing,
  deduplicated with Windows path identity, reusable with one click, removable
  individually or clearable without affecting files or active archive state.
- Align the README, bilingual privacy policy, and desktop documentation with
  recent-path persistence, including local retention, user controls, and the
  explicit statement that hexadecimal field encoding is not encryption.
- Add explicit Select all, Select none, and Invert selection actions to both
  archive views, with `Ctrl+I`, bilingual status feedback, semantic shortcut
  metadata, and a 100,000-entry inversion benchmark.
- Strengthen the standalone x64 Release EXE smoke from read-only listing to a
  create-list-extract ZIP round trip in a directory without a separate Worker.

## [0.1.6] - 2026-08-31

### Added

- Add `Ctrl+R` to reload the current archive in both desktop UIs, expose the
  shortcut on semantic reload controls, and include it in bilingual shortcut
  help and user documentation.
- Add `Ctrl+F` to move directly to the loaded archive's search field in both
  desktop UIs while preserving the current query, with semantic metadata and
  bilingual help.
- Add an explicit Close archive action and `Ctrl+W` to both desktop UIs. Closing
  returns home and releases the archive password, metadata, selection, search,
  and folder-navigation session state; active work prevents closure.
- Add one-click extraction to a matching sibling folder in both desktop UIs,
  reusing the shell command's compound-extension naming, conflict policy,
  operation queue, isolated Worker, and completed-output discovery.

## [0.1.5] - 2026-08-31

### Added

- Show non-empty create-source validation failures inline in the Iced create
  page while keeping the Create action disabled until the sources are valid.
- Add a bilingual keyboard-shortcut reference to both desktop About pages for
  open, create, archive selection, help, and operation cancellation. Regression
  tests keep the visible key combinations aligned with the implemented maps.
- Add a bilingual status-bar action in both desktop UIs that reveals the most
  recent successfully created archive or extraction destination in File
  Explorer. New work, failures, and protocol mismatches clear the action.

### Changed

- Generate and verify WinGet candidates from the same public all-in-one MSIX
  bundle used by GitHub Releases, without unpublished per-architecture MSIX
  inputs.
- Clear the create-form password in both desktop UIs immediately after a create
  request is accepted for execution or queuing, while retaining it when a full
  queue rejects the request so the user can retry.
- Retry the pinned WinGet validation-client repair up to three times with
  bounded backoff for transient CDN connection failures, and record the retry
  limit in validation evidence.

## [0.1.4] - 2026-08-30

### Added

- Show non-empty create-source validation failures inline in the Iced create
  page while keeping the Create action disabled until the sources are valid.
### Changed

- Retry the pinned WinGet validation-client repair up to three times with
  bounded backoff for transient CDN connection failures, and record the retry
  limit in validation evidence.
- Keep public release assets limited to the all-in-one MSIX bundle, standalone
  x64/ARM64 executables, and `SHA256SUMS.txt`.

## [0.1.3] - 2026-08-30

### Added

- Publish a genuinely standalone Windows desktop EXE for x64 and ARM64. The
  desktop binary embeds the Worker runtime and relaunches itself in protocol
  mode, so portable users no longer need a second Worker file.

### Changed

- Restrict public GitHub Release assets to one all-in-one MSIX bundle, one
  standalone EXE per architecture, and `SHA256SUMS.txt`.
- Keep CLI, Worker, Shell DLL, audits, SBOMs, provenance, and WinGet YAML as
  build or signing evidence instead of user-facing Release assets.

## [0.1.2] - 2026-08-30

### Added

- Publish one all-in-one Windows MSIX bundle plus architecture-matched portable
  desktop and Worker executables for users who do not want to install MSIX.
- Keep GitHub Release assets focused on the bundle, portable executables, and a
  single SHA-256 manifest; build audits, SBOMs, provenance, and WinGet YAML stay
  in workflow evidence artifacts.

## [0.1.0] - 2026-08-29

### Added

- First usable public Windows release of ZiFile with ZIP, 7z, TAR families,
  common single-stream formats, read-only RAR/CAB support, Explorer commands,
  bilingual desktop UI, CLI, isolated Worker, and x64/ARM64 artifacts.
- Public GitHub Release automation that publishes unsigned Windows packages,
  standalone executables, checksums, SBOMs, audits, and WinGet manifest candidates
  directly from a matching version tag.

### Changed

- Stabilized the archive browser, cancellation and FIFO operation queue, and
  refreshed the desktop visual hierarchy for a more professional Windows UI.

## [0.1.0-alpha.1] - 2026-08-28

### Added

- Initial Windows alpha milestone for the Rust workspace, including the Iced
  desktop UI, CLI, isolated Worker, and Astro Starlight documentation site.
- ZIP, 7z, TAR-family, gzip, Zstandard, XZ, Bzip2, LZ4, Brotli, RAR
  read-only beta, and CAB read-only beta archive workflows.
- Bilingual archive browsing, creation, extraction, integrity testing, search,
  selection, pagination, conflict policies, password unlock, and cancellation.
- Windows x64/ARM64 packaging, checksums, SBOM, provenance, and staged GitHub
  Release automation.

### Fixed

- Stabilized the accessible keyboard workflow across asynchronous archive load,
  reload, navigation, selection, and create-form state changes.
- Recomputed stable-release checksums after cloud signing replaces the staged
  executable and package files.

## [0.1.1] - 2026-08-30

### Added

- Add explicit bilingual `Extract all` actions to both desktop archive views,
  while keeping `Extract selected` available for precise partial extraction.
- Give the default desktop window a centered startup position and a 920×620
  minimum size so archive tables and creation controls remain usable when the
  window is resized.
- Add an explicit bilingual checksum-copy action to both desktop archive tables,
  with clipboard failure surfaced as an error status.
- Add a bilingual `Show in File Explorer` action to both desktop archive
  headers, selecting the currently open archive in its containing folder.
- Keep the accessible archive browser lightweight during high-frequency progress
  updates for large archives, while retaining queue, cancel, open, and reveal actions.
- Keep the desktop queue in its busy state while handing work directly from a
  completed Worker to the next queued operation, avoiding a transient full-table
  render between large-archive operations.
- Added independent standalone LZMA creation and detection through `lzma-rust2`; `.lzma` no longer shares the user-visible XZ format label.
- Keep formal Store identity values out of unsigned prerelease and manual
  validation packages; only stable tags and explicit cloud-signing rehearsals
  pass them to the MSIX builder, preserving the development identity elsewhere.
- Automatically add the selected format's canonical extension when a save
  dialog name has no extension, consistently across both desktop UIs while
  preserving an explicit user-entered extension.
- Keep the Iced and Dioxus create pages aligned by reporting source additions,
  removals, and clears through shared bilingual status copy.
- Give the default Iced operation footer explicit informational and error
  states, with danger styling for worker failures and invalid operations.
- Correct the archive empty state so an in-progress Worker load is shown as
  opening instead of being reported as a failure in either desktop UI.
- Align Iced search feedback with the accessible candidate by showing match
  counts and removing the misleading “Page 1 / 1” display for zero results.
- Added decoded-content SHA-256 checksums to archive integrity-test results;
  the CLI prints a stable checksum table and both desktop archive tables show
  the values after a successful test.
- Hardened the Windows 11 Explorer context-menu extension with bilingual
  tooltips, filesystem-path validation for create commands, and the required
  `E_PENDING` fast path when Explorer disallows slow state evaluation.
- Added the create command to Windows 11 folder-background menus so users can
  archive the current folder without first selecting an item.
- Extended the Explorer extract command to EPUB files, matching the core's
  signature-first ZIP inspection without claiming `.epub` as the default app.
- Added COM object and `LockServer` lifetime accounting so Explorer can unload
  the shell DLL when no command, factory, or server lock remains.
- Made folder-background creation robust when Explorer supplies an empty item
  array by resolving the current folder through the Explorer site instead of
  assuming a selected path.
- Deduplicate Explorer create sources inside the Shell extension before
  launching the desktop, using the same Windows case- and slash-insensitive
  path identity rule while keeping extract invocation strict about one item.
- Apply the Explorer command-line budget after create-source deduplication, so
  repeated spellings cannot cause a false over-limit rejection.
- Apply the same command-line budget to folder-background creation, so an
  unusually deep current folder is rejected before a desktop process starts.
- Align Explorer Shell state checks with core path safety by rejecting junctions
  and Windows reparse-point sources before the context-menu command is enabled.
- Apply the same junction/reparse-point source check to both desktop create
  forms before they open the save dialog, keeping UI preflight and Worker
  behavior consistent.
- Made archive creation refuse an existing output file with a structured
  destination-conflict error, preserving the existing file across all
  temporary-file-backed creators while extraction keeps its explicit conflict
  policy.
- Made the real foreground queue smoke deterministic with a bounded test-only
  Worker startup delay and fresh UI Automation button lookup after redraws;
  cancellation remains observed during the delay and production defaults stay
  disabled.
- Preserve the original output stem when extracting legacy `.lzma` and `.bz`
  single-stream aliases instead of appending an unnecessary `.out` suffix.
- Decode actual LZMA-alone `.lzma` streams with the pure-Rust `lzma-rust2`
  provider instead of routing them through the XZ decoder.
- Add TAR + LZMA-alone (`.tar.lzma`) creation, listing, integrity testing,
  extraction, and CLI/UI format selection with the same safety limits as other
  TAR compositions.
- Execute Windows Criterion format-detection and archive-throughput benchmarks
  in a bounded CI job and retain their performance evidence for 30 days.
- Include TAR + LZMA-alone creation and integrity throughput in the core
  benchmark suite using a bounded 1 MiB sample.
- Add bidirectional TAR + LZMA-alone interoperability against Windows
  `tar.exe`, with a structured no-user-data CI evidence artifact.
- Reject empty Markdown pages in the bilingual documentation locale-parity
  check, so a present but blank mirror cannot pass documentation CI.
- Centralize the stable creation-format order in the core capability registry so
  the Iced and accessible desktop menus cannot drift apart as providers evolve.
- Reuse the core format capability registry in the Explorer extract command and
  require a real file, preventing archive-looking directories from showing a
  misleading extraction action.
- Reject symbolic links, junctions, and Windows reparse points in extraction
  destinations and existing output parents; preflight a non-directory target
  before parsing the archive so invalid destinations fail early and cannot
  redirect output outside the selected folder.
- Block creation before the save dialog when a previously selected source has
  disappeared, with bilingual recovery guidance in both desktop UIs.
- Share signature-first drag-and-drop classification between both desktop UIs,
  while retaining extension fallback for formats without universal magic bytes.
- Run drag-and-drop probing off the UI event thread in both desktop variants,
  so slow or remote files cannot block the window while they are classified.
- Move open, extraction-destination, source-selection, and archive-save dialogs
  off the UI event thread in both desktop variants, with a re-entry guard that
  prevents duplicate native dialogs while one is active.
- Share Windows-aware source path deduplication between both desktop variants,
  preventing duplicate roots when Explorer, file pickers, or drag-and-drop use
  different casing or slash separators.
- Apply Unicode-aware Windows path normalization so non-ASCII case variants are
  deduplicated consistently with ordinary Latin paths.
- Align matching-folder naming with the core format registry so unsupported
  `.tar.lz4`, `.tar.br`, and `.tar.bz` combinations are not presented as
  supported extraction formats.
- Centralize supported TAR compound suffixes and aliases in the core registry,
  so format detection and Explorer matching-folder naming cannot drift apart.
- Check cancellation before creating archive parents or extraction destinations,
  avoiding avoidable empty directories when an operation is already cancelled.
- Return a stable destination-conflict error when extraction is pointed at an
  existing file, preserving that file instead of exposing a raw directory I/O error.
- Treat Unicode case variants as one archive entry name on Windows, preventing
  entries such as `Ä.txt` and `ä.txt` from colliding only after extraction.
- Reject symbolic-link creation sources in both desktop forms before opening the
  save dialog, matching the core link-entry policy with bilingual guidance.
- Replace completed temporary archive outputs atomically instead of deleting the
  existing file first, preserving the previous file if the final replacement fails.
- Include the common `tar.gz`, `tar.zst`, `tar.xz`, and `tar.bz2` compound suffixes
  in the shared open-file dialog filter so supported TAR compositions are discoverable.
- Extend the packaging policy smoke to explicitly lock those compound suffixes in
  the core open-extension contract.
- Expand the Windows Foundation smoke into a real CLI round-trip matrix for
  TAR+Zstandard, TAR+XZ, TAR+Bzip2, gzip, Zstandard, XZ, Bzip2, LZ4, and Brotli;
  each case now creates, integrity-tests, extracts, and checks Unicode or stream
  output content.
- Detect renamed TAR+gzip, TAR+Zstandard, TAR+XZ, and TAR+Bzip2 archives by
  probing a bounded decoded TAR header, keeping drag-and-drop, Explorer, and
  CLI opening behavior consistent without scanning an unbounded input.
- Route stable Worker errors through the shared bilingual formatter in both
  desktop UIs, translating password, unknown-format, destination-conflict,
  safety-limit, unsupported-operation, and cancellation messages while keeping
  backend-specific diagnostic details intact.
- Reject archive destinations that are equal to or inside a source tree before
  creating its parent or temporary output, preventing self-inclusion and
  leaving no directory side effect on this recoverable input error.
- Use overflow-free exact arithmetic for expansion-ratio limits so fractional
  boundary cases cannot be rounded down by integer division.
- Apply the configured expansion-ratio limit to single-stream listing without
  silently widening a zero limit to one compressed byte of output.
- Check cumulative TAR entry sizes against expanded-size and ratio limits
  immediately after each header is parsed, before compressed payloads are skipped.
- Persist language and theme preferences through a flushed, durable temporary
  file replacement on Windows, and surface save failures in both desktop UIs.
- Inject the fixed SignPath Foundation attribution into every generated stage
  prerelease Release Notes entry while retaining GitHub's generated changes.
- Keep the Worker progress stream continuous through archive preflight while
  resetting counters cleanly when integrity testing or extraction begins.
- Add a localized Clear search action to the default Iced archive browser and
  reuse the same label in the accessible UI.
- Show a localized empty-state message when an archive folder or search has no
  visible entries in either desktop UI.
- Distinguish password-related open failures from corrupt, unsupported, or
  ordinary I/O failures so only the former presents an Unlock retry flow.
- Ignore stale Worker completions before they can mutate either desktop UI's
  active operation state or archive contents.

### Fixed

- Align both desktop create forms for single-stream formats: Add files opens
  a single-file picker, Add folder is rejected defensively, and Dioxus create
  preflight failures now use error semantics instead of informational status.
- Announce the selected create format's source requirements in the accessible
  Dioxus status region, matching the default Iced form when the format changes.
- Cover create-format state transitions with direct tests for level clamping,
  password clearing, and bilingual source-requirement status updates.
- Keep the currently visible archive while a replacement open request is
  queued or rejected by a full operation queue; the Iced view now switches
  only when the queued Worker actually starts.
- Make the Explorer extract command use signature-first format detection during
  permitted slow state evaluation, so renamed valid archives are discoverable and
  invalid files with a forged archive suffix stay hidden.

- Move keyboard focus to the labelled main region after accessible-desktop page
  changes, while leaving focus untouched during progress and status-only renders.
- Require exact modifier combinations for default Iced desktop shortcuts so
  extra Shift, Alt, or Windows modifiers retain their native behavior.
- Match accessible-desktop shortcuts against their exact non-lock modifiers so
  combinations such as `Ctrl+Shift+N`, `Alt+F1`, and `Ctrl+Shift+A` retain their
  native behavior instead of invoking ZiFile's simpler declared commands.
- Preserve native Escape behavior while the accessible desktop is idle, and
  intercept Escape only when an active operation has a cancellation token.
- Expose every handled accessible-desktop shortcut (`Ctrl+O`, `Ctrl+N`, `F1`,
  `Escape`, and archive-scoped `Ctrl+A`) on its corresponding semantic control
  instead of leaving open/create shortcuts discoverable only from documentation.
- Use a two-tone, theme-aware keyboard focus indicator in the accessible desktop
  candidate so focus remains visible on both light surfaces and cyan active controls,
  while forced-colors mode continues to use Windows system colors.
- Require WACK readiness to compare the package audit against all three exact Partner Center identity fields and persist structured failures for incomplete audits, preventing a signed or malformed test identity from entering formal certification preflight.
- Keep WinGet file-extension metadata synchronized with all 29 archive extensions accepted by the desktop open workflow, including RAR, CAB, ZIPX, comic-book aliases, and TAR stream aliases.

### Added

- Replace the single-resolution desktop icon with a deterministic Win32 ICO
  containing 16, 24, 32, 48, and 256 pixel 32-bit PNG frames. The reviewed
  catalog pins its hash, binary validation rejects malformed frame directories,
  and package audit rechecks both the unpacked icon and the compiled desktop
  executable's `GROUP_ICON`/`ICON` resources for every architecture.
- Add the complete reviewed MSIX high-DPI icon matrix: required Store scale
  variants, 100/200/400 percent application and medium-tile assets, and 14
  taskbar/Start target sizes in default, dark-unplated, and light-unplated forms.
  CI pins all 58 PNG hashes, verifies x64 generator output, rejects missing or
  modified qualified resources, and rechecks the assets inside built packages.
- Add a reviewed 300x300 Microsoft Store app tile listing icon with a
  machine-readable manifest, pinned hash, generator check, and negative policy
  fixtures for missing, resized, or modified assets.
- Both desktop interfaces now include a bilingual About page with the running
  package version, MIT license, supported format-family count, project address,
  and local-processing privacy boundary; the accessible candidate uses semantic
  definition-list markup, a responsive single-column layout, and an exposed `F1`
  shortcut.
- Bilingual owner-facing Store and WinGet onboarding guidance now records the
  Company-account boundary, current fee and name-reservation window, business
  verification inputs, and the first-contribution GitHub/CLA requirements.
- Formal Partner Center packaging now injects and audits the exact Publisher
  Display Name in addition to Identity and Publisher; the trusted lifecycle
  workflow downloads signed rather than pre-signing x64 artifacts.
- Bilingual end-user getting-started and troubleshooting guides covering safe
  extraction, format input rules, queue/cancellation behavior, CLI passwords,
  Worker failures, development-package trust, and responsible issue reporting.
- Rust workspace with `zifile-core`, `zifile-cli`, and `zifile-desktop`.
- Shared archive format capability registry and extension detection.
- Conservative default extraction limits.
- Iced desktop technology shell.
- Astro Starlight documentation, roadmap, ADRs, and Stage 0 work log.
- Unit, benchmark, smoke, CI, documentation, and release foundations.
- Packaging-policy smoke coverage locks the x64/ARM64 reproducibility workflow's
  hard timeout, stale-run cancellation, independent matrix results, and retained
  failure evidence so a long double build cannot wait without a bounded outcome.
- The operation-queue foreground smoke now proves the ZiFile native window owns
  the Windows foreground before invoking UI Automation and includes bounded last
  document text in timeout diagnostics instead of treating a background run as
  real foreground evidence.
- Extend the accessible keyboard smoke with a deterministic two-entry archive
  workflow covering integrity testing, reload, scoped search and selection,
  conflict policy, pagination state, and extract-button state; retain an
  explicit legacy create-form isolation switch for focused diagnostics.
- Dual-architecture Release packaging jobs now have a 90-minute hard timeout,
  enforced by packaging-policy smoke coverage, so tests or MSIX builds cannot
  leave a manual rehearsal running without a bounded result.
- Every CI, documentation deployment, SBOM, and GitHub Release publication job
  now has a workload-sized hard timeout, with job-scoped policy assertions that
  prevent an unrelated timeout elsewhere in the YAML from satisfying the gate.
- Real ZIP/ZIP64/AES and 7z/AES create, list, verify, and extraction operations.
- Format-aware compression controls: 7z creation now applies the selected LZMA2
  level, Zstandard and Brotli expose their full ranges, Bzip2 enforces its valid
  minimum, fixed-level TAR/LZ4 creation no longer presents an inert slider, and
  the CLI advertises format-specific ranges, rejects out-of-range values, and
  refuses explicit levels for fixed-setting formats.
- Encrypted 7z/RAR header retry views in both desktop interfaces, including correct
  7z AES entry flags after a password-protected archive is unlocked.
- TAR, tar.gz, tar.zst, tar.xz, tar.lzma and tar.bz2 archive compositions.
- gzip, Zstandard, XZ, Bzip2, LZ4 and Brotli single-stream operations.
- Signature-based detection and a shared safe extraction policy covering traversal,
  links, Windows device names, case collisions, conflicts and expansion limits.
- CLI archive commands and a modern Iced archive browser/creator with background work.
- Desktop drag-and-drop opens known archives or adds files and folders as creation sources.
- Determinate byte/entry progress, cooperative cancellation, and bounded list-time decoding.
- Source modification-time preservation for ZIP creation and ZIP/7z/TAR/RAR/CAB extraction,
  plus a timezone-honest Modified column in both desktop archive browsers.
- Deterministic Windows assets, x64/ARM64 MSIX packaging and archive file associations.
- Tag-driven checksums, CycloneDX SBOM, provenance and WinGet 1.12 manifest generation.
- A release-blocking WinGet candidate verifier that enforces the community-repository
  path, four-file schema layout, versioned official URLs, dual architectures, and
  exact SHA-256 matches against the signed local MSIX packages, plus official
  `winget validate` schema checks in Windows CI.
- Store privacy-route gates that verify both localized Astro outputs during CI and
  the deployed public HTTPS pages after every GitHub Pages publication.
- Security-focused fuzz targets and archive throughput benchmarks.
- Permanent malformed/truncated-header regression coverage for all 16 supported archive
  and compression format classes, requiring both list and integrity-test rejection.
- Bidirectional ZIP, tar.gz, tar.lzma and 7z interoperability tests against Windows reference tools.
- Simplified Chinese and English desktop UI with system-locale detection and persisted
  language/theme preferences; passwords are never included in settings.
- Archive-path search and bounded 500-row pagination, with a 100,000-entry regression test.
- Sortable archive columns for name, original size, packed size, and modified time,
  with folder-first ordering, accessible direction state, and bounded 500-row pages.
- Hierarchical archive folder navigation in both desktop UIs, including synthesized
  implicit directories, root-to-current breadcrumbs, and archive-wide search that
  retains full paths while folder pages remain bounded to 500 rows.
- Recursive folder selection in both archive browsers, with one-pass descendant
  aggregation, bilingual selected/total feedback, accessible mixed state in the
  Dioxus candidate, and deterministic handling of file/folder path conflicts.
- Desktop shortcuts for opening (`Ctrl+O`), creating (`Ctrl+N`), selecting all (`Ctrl+A`)
  and canceling an active operation (`Escape`).
- Bidirectional 7z interoperability against Windows bsdtar/libarchive.
- Weekly bounded fuzz campaigns with retained crash artifacts on failure.
- Versioned, line-delimited IPC and a dedicated archive Worker process for all desktop
  list, test, extract and create operations.
- Windows Job Object containment: one active process, 4 GiB process-memory ceiling,
  kill-on-close behavior, cooperative create/extract cancellation and timed forced
  reclamation as a fallback.
- Windows taskbar states driven by Worker progress, including indeterminate and
  cancelling states, plus a packaged `zifile.exe` App Execution Alias.
- Opt-in Dioxus/WebView2 accessibility candidate, written in Rust RSX, with semantic
  navigation, archive tables, native form controls, live status/progress, bilingual
  themes, command-line archive opening, and Worker-backed list/test/extract/create flows.
- Candidate-native file drop handling, Ctrl+O/Ctrl+N/Escape shortcuts, a local-only
  WebView CSP, and opt-in x64/ARM64 MSIX validation artifacts for manual release runs.
- Candidate archive-scoped Ctrl+A with dynamic UI Automation selection labels and
  forced-colors styling for Windows high-contrast modes.
- Shared 100,000-entry browser benchmarks plus a repeatable Windows startup and
  desktop-process-tree memory baseline script.
- Real 100,000-entry ZIP UI exercise through the isolated Worker, including a
  bounded 500-row UI Automation tree, search, pagination and process-tree memory sampling.
- Deterministic 100,000-entry archive-browser instrumentation for first content,
  50% scrolling, pagination and simultaneous root-plus-descendant peak memory.
- WinGet candidate validation with real x64/ARM64 artifact hashes, unsigned MSIX
  publisher-namespace guards, and an ephemeral self-signed package-signing check.
- Deterministic 100,000-entry desktop load-cancellation instrumentation that verifies
  final UI status, acknowledgement latency, Worker exit and temporary-fixture cleanup.
- Foreground-safe Windows keyboard regression coverage for bilingual candidate navigation,
  create-form selects/ranges/passwords, reverse traversal and disabled-control skipping.
- A bounded 32-operation FIFO shared by both desktop UIs, with monotonic completion IDs,
  active cancellation, pending-work clearing, and non-persistent sensitive payloads.
- Entry/byte progress and cooperative cancellation for archive integrity testing across
  ZIP, 7z, RAR, CAB, TAR compositions, and supported compression streams.
- Entry-scan progress and cooperative cancellation while opening all supported archives,
  including expanded-byte scan feedback for single compression streams and bilingual
  indeterminate scanning copy until a final total is known.
- A pure-Rust read-only RAR 1.3–7 beta provider with encrypted headers, solid archives,
  selected extraction, resource limits, cancellation, and link/redirection rejection.
- A pure-Rust read-only Windows CAB beta provider with signature detection, browsing,
  integrity testing, selective safe extraction, cancellation, resource limits, and
  explicit rejection of unsupported multi-cabinet sets; corrupt compressed data fails
  integrity testing and cannot commit partial extraction output.
- A Rust Windows 11 `IExplorerCommand` extension and shared `--create` startup protocol,
  packaged behind trusted-install and Explorer-activation release gates.
- Store listing, privacy, screenshot-import, WACK-readiness, signed-package lifecycle,
  package-audit, and release-signing policy gates with structured evidence.
- Candidate CLI and core-provider compatibility documentation with exact regression
  coverage for commands, value names, exit codes, creation-input capabilities, and
  compression-level ranges, including executable CLI smoke coverage.

### Changed

- The CLI accepts archive passwords only through opt-in standard input rather than a
  plaintext command-line argument.
- Archive parsing applies caller safety limits while listing and converts recoverable
  7z backend failures into ordinary errors; historical fuzz findings are permanent tests.
- Rust is pinned to 1.93.0 and the 7z backend to `sevenz-rust2` 0.22.0.
- Windows builds use `/Brepro`, one Cargo job, and isolated-path remapping; x64 and ARM64
  reproducibility gates compare five PE artifacts and currently pass 5/5.
- Both desktop UIs preflight creation sources, distinguishing multi-source archives from
  single-file stream formats before opening a save dialog or starting the Worker.
- The Cargo workspace version is the single source for packages, documentation, tags,
  Release runs, and deterministic four-part MSIX version mapping.
