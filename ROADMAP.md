# ZiFile roadmap

This roadmap is the source of truth for planned delivery. GitHub milestones and
the Starlight roadmap page must remain synchronized with it.

## Stage 0 — Foundation

Goal: prove the product identity, repository structure, Rust UI direction,
documentation system, and engineering gates.

- [x] Reserve the `ax2/zifile` repository name locally and define product identity.
- [x] Create a Rust 2024 workspace with desktop, CLI, and core crates.
- [x] Add an Iced desktop technology shell.
- [x] Add a shared format capability registry and conservative safety limits.
- [x] Add initial unit test, benchmark, smoke, CI, and release structure.
- [x] Create the Astro Starlight documentation system.
- [x] Bound a 100,000-entry Iced listing with search and 500-row pagination.
- [x] Add the first keyboard workflow: Ctrl+O, Ctrl+N, Ctrl+A and Escape.
- [ ] Verify Chinese IME, full keyboard navigation, screen readers, per-monitor DPI and high contrast ([#14](https://github.com/ax2/zifile/issues/14)).
- [ ] Reserve `ZiFile` in Microsoft Partner Center ([#8](https://github.com/ax2/zifile/issues/8)).
- [ ] Decide signing provider for direct GitHub and WinGet releases ([#9](https://github.com/ax2/zifile/issues/9)).

Exit criteria: the workspace and docs build from a clean checkout; the UI spike
records an evidence-backed keep/replace decision and migration path; open
architecture risks have ADRs.

## Stage 1 — Alpha

Goal: safely open, list, create, and extract common archives.

- [x] ZIP read/write, including ZIP64 and AES-encrypted entries.
- [x] 7z read/write with optional AES encryption.
- [x] Beta read-only RAR 1.3–7 browse/test/selective extraction with password support; creation stays disabled.
- [x] TAR and gzip/zstd/xz/bzip2 stream composition; lz4/brotli are available as single streams.
- [x] Signature-based format detection.
- [x] Archive browser, selected extraction, destination and conflict policies.
- [x] Background execution with cooperative cancellation.
- [x] Determinate byte/entry progress for extraction and creation.
- [ ] Queued multi-operation scheduling (bounded FIFO scheduler and both UI
  integrations implemented; real foreground multi-operation smoke pending in
  [#11](https://github.com/ax2/zifile/issues/11)).
- [x] Path traversal, link escape, reserved-name, collision, and expansion-limit defenses.
- [x] Fuzz targets compile continuously in CI.
- [x] ZIP, tar.gz and 7z bidirectional interoperability with Windows reference tools.
- [x] Run bounded path-policy and format-detection fuzz campaigns on a weekly schedule.
- [x] Add a bounded parser fuzz target for every supported archive and stream format.
- [x] Broader third-party 7-Zip/libarchive corpus expansion.

Exit criteria: ZIP and TAR-family round trips interoperate with reference tools;
all security fixtures pass; no archive task blocks the UI thread.

## Stage 2 — Beta

Goal: complete the Windows-focused daily workflow.

- [x] 7z read/write and AES encryption delivered ahead of plan.
- [x] Password flow avoids logging or persisting secrets.
- [x] Windows archive file associations and desktop drag-and-drop.
- [x] Windows taskbar progress and packaged `zifile.exe` App Execution Alias.
- [ ] Windows 11 File Explorer `IExplorerCommand` context menu extension (Rust DLL and MSIX
  registration implemented; trusted-package Explorer activation tracked by
  [#12](https://github.com/ax2/zifile/issues/12)).
- [x] Isolated worker process with versioned IPC, cancellation and Windows Job Object limits.
- [x] x64/ARM64 MSIX and standalone executable packaging.
- [ ] Signed install, upgrade, repair and uninstall tests ([#12](https://github.com/ax2/zifile/issues/12); trusted-package lifecycle harness
  covers install, CLI launch, upgrade, guaranteed uninstall, Reset, and a Windows App SDK Repair
  probe with LocalState preservation evidence when supported; real signed execution remains pending,
  and current Windows 25H2 reports the distinct Repair API unsupported).
- [x] Establish throughput, 100,000-entry UI-model, idle startup and process-tree memory baselines.
- [x] Exercise Worker listing, bounded first-page rendering, search and pagination with a real 100,000-entry archive.
- [x] Add repeatable first-content/scroll latency and simultaneous process-tree peak-memory instrumentation for the real 100,000-entry archive.
- [x] Exercise desktop-to-Worker cancellation while a real 100,000-entry archive is loading, including acknowledgement latency and Worker reclamation.

Exit criteria: signed Beta packages install, upgrade, repair, and uninstall on
supported Windows 10/11 environments.

## Stage 3 — Release candidate

Goal: distribution, accessibility, localization, and hardening.

- [x] Simplified Chinese and English desktop UI with persisted language/theme preferences.
- [x] Build an opt-in Dioxus/WebView2 accessibility candidate with semantic archive and create workflows over the isolated Worker.
- [x] Add candidate CSP/offline resource policy, native drop handling, core shortcuts and local x64 MSIX validation.
- [x] Verify candidate archive-scoped Ctrl+A and dynamic selection labels in a real UI Automation session.
- [x] Add repeatable bilingual keyboard traversal for candidate navigation and the native create form, including disabled-control skipping and foreground-window protection.
- [x] Cross-build, package, attest and checksum the candidate MSIX and executables for x64 and ARM64.
- [ ] Run the candidate MSIX and runnable directory on physical ARM64 Windows hardware ([#13](https://github.com/ax2/zifile/issues/13)).
- [x] Complete equivalent Simplified Chinese and English product documentation.
- [ ] Complete full keyboard traversal, Narrator, Accessibility Insights, high-contrast, IME and per-monitor DPI verification before replacing Iced ([#14](https://github.com/ax2/zifile/issues/14)).
- [ ] Capture and validate the formal bilingual Store screenshot set from one signed candidate ([#15](https://github.com/ax2/zifile/issues/15)).
- [ ] Windows Application Certification Kit run ([#16](https://github.com/ax2/zifile/issues/16)).
- [ ] WinGet manifest submission ([#18](https://github.com/ax2/zifile/issues/18)).
- [ ] Microsoft Store submission and certification ([#17](https://github.com/ax2/zifile/issues/17)).
- [x] Tag workflow for SBOM, provenance attestation and checksums.
- [x] Reproducible-build documentation and comparison evidence (toolchain pin,
  deterministic linking, target-path remapping, schema v2 diagnostics, and
  clean x64/ARM64 5/5 evidence completed in run 32826187552).

## Stage 4 — 1.0

Goal: stable public release.

- [ ] Freeze public CLI and provider contracts ([#19](https://github.com/ax2/zifile/issues/19)); candidate command/value/exit-code and provider compatibility boundaries are documented and machine-tested, with final freeze reserved for the 1.0 release commit.
- [ ] Resolve release-blocking compatibility and accessibility issues ([#19](https://github.com/ax2/zifile/issues/19)).
- [ ] Complete user, security, contributor, and release documentation ([#19](https://github.com/ax2/zifile/issues/19)).
- [ ] Publish GitHub, WinGet, and Microsoft Store releases from one version source ([#19](https://github.com/ax2/zifile/issues/19)).

## Post-1.0

- Broader RAR multivolume and recovery-record compatibility after the beta read-only provider stabilizes.
- Checksums, duplicate analysis, batch rename, and file preview.
- Optional stronger worker isolation with AppContainer.
- ARM64 performance tuning and expanded enterprise deployment guidance.
