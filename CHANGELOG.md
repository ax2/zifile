# Changelog

All notable changes to ZiFile are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

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
- Keep WinGet file-extension metadata synchronized with all 24 archive extensions accepted by the desktop open workflow, including RAR, CAB, ZIPX, comic-book aliases, and TAR stream aliases.

### Added

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
- TAR, tar.gz, tar.zst, tar.xz and tar.bz2 archive compositions.
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
- Permanent malformed/truncated-header regression coverage for all 15 supported archive
  and compression format classes, requiring both list and integrity-test rejection.
- Bidirectional ZIP and tar.gz interoperability tests against Windows reference tools.
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
