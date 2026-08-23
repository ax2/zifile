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
- [ ] Verify Chinese IME, full keyboard navigation, screen readers, per-monitor DPI and high contrast.
- [ ] Reserve `ZiFile` in Microsoft Partner Center.
- [ ] Decide signing provider for direct GitHub and WinGet releases.

Exit criteria: the workspace and docs build from a clean checkout; the UI spike
passes its acceptance checklist; open architecture risks have ADRs.

## Stage 1 — Alpha

Goal: safely open, list, create, and extract common archives.

- [x] ZIP read/write, including ZIP64 and AES-encrypted entries.
- [x] 7z read/write with optional AES encryption.
- [x] TAR and gzip/zstd/xz/bzip2 stream composition; lz4/brotli are available as single streams.
- [x] Signature-based format detection.
- [x] Archive browser, selected extraction, destination and conflict policies.
- [x] Background execution with cooperative cancellation.
- [x] Determinate byte/entry progress for extraction and creation.
- [ ] Queued multi-operation scheduling.
- [x] Path traversal, link escape, reserved-name, collision, and expansion-limit defenses.
- [x] Fuzz targets compile continuously in CI.
- [x] ZIP, tar.gz and 7z bidirectional interoperability with Windows reference tools.
- [x] Run bounded path-policy and format-detection fuzz campaigns on a weekly schedule.
- [ ] Broader third-party 7-Zip/libarchive corpus and parser fuzz targets.

Exit criteria: ZIP and TAR-family round trips interoperate with reference tools;
all security fixtures pass; no archive task blocks the UI thread.

## Stage 2 — Beta

Goal: complete the Windows-focused daily workflow.

- [x] 7z read/write and AES encryption delivered ahead of plan.
- [x] Password flow avoids logging or persisting secrets.
- [x] Windows archive file associations and desktop drag-and-drop.
- [x] Windows taskbar progress and packaged `zifile.exe` App Execution Alias.
- [ ] Windows 11 File Explorer `IExplorerCommand` context menu extension.
- [x] Isolated worker process with versioned IPC, cancellation and Windows Job Object limits.
- [x] x64/ARM64 MSIX and standalone executable packaging.
- [ ] Signed install, upgrade, repair and uninstall tests.
- [ ] Performance baselines for throughput, memory, startup, and large listings.

Exit criteria: signed Beta packages install, upgrade, repair, and uninstall on
supported Windows 10/11 environments.

## Stage 3 — Release candidate

Goal: distribution, accessibility, localization, and hardening.

- [x] Simplified Chinese and English desktop UI with persisted language/theme preferences.
- [x] Build an opt-in Dioxus/WebView2 accessibility candidate with semantic archive and create workflows over the isolated Worker.
- [x] Add candidate CSP/offline resource policy, native drop handling, core shortcuts and local x64 MSIX validation.
- [x] Cross-build, package, attest and checksum the candidate MSIX and executables for x64 and ARM64.
- [ ] Run the candidate MSIX and runnable directory on physical ARM64 Windows hardware.
- [ ] Complete equivalent Simplified Chinese and English product documentation.
- [ ] Complete keyboard-only (including Ctrl+A), Narrator, Accessibility Insights, high-contrast, IME and per-monitor DPI verification before replacing Iced.
- [ ] Windows Application Certification Kit run.
- [ ] WinGet manifest submission.
- [ ] Microsoft Store submission and certification.
- [x] Tag workflow for SBOM, provenance attestation and checksums.
- [ ] Reproducible-build documentation and comparison evidence.

## Stage 4 — 1.0

Goal: stable public release.

- Freeze public CLI and provider contracts.
- Resolve release-blocking compatibility and accessibility issues.
- Complete user, security, contributor, and release documentation.
- Publish GitHub, WinGet, and Microsoft Store releases from one version source.

## Post-1.0

- RAR read-only compatibility after a license and implementation review.
- Checksums, duplicate analysis, batch rename, and file preview.
- Optional stronger worker isolation with AppContainer.
- ARM64 performance tuning and expanded enterprise deployment guidance.
