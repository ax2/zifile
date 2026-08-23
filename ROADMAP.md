# ZiFile roadmap

This roadmap is the source of truth for planned delivery. GitHub milestones and
the Starlight roadmap page must remain synchronized with it.

## Stage 0 — Foundation (current)

Goal: prove the product identity, repository structure, Rust UI direction,
documentation system, and engineering gates.

- [x] Reserve the `ax2/zifile` repository name locally and define product identity.
- [x] Create a Rust 2024 workspace with desktop, CLI, and core crates.
- [x] Add an Iced desktop technology shell.
- [x] Add a shared format capability registry and conservative safety limits.
- [x] Add initial unit test, benchmark, smoke, CI, and release structure.
- [x] Create the Astro Starlight documentation system.
- [ ] Validate Iced with 100,000 rows, IME, keyboard navigation, DPI, and high contrast.
- [ ] Reserve `ZiFile` in Microsoft Partner Center.
- [ ] Decide signing provider for direct GitHub and WinGet releases.

Exit criteria: the workspace and docs build from a clean checkout; the UI spike
passes its acceptance checklist; open architecture risks have ADRs.

## Stage 1 — Alpha

Goal: safely open, list, create, and extract common archives.

- ZIP read/write, including ZIP64.
- TAR and gzip/zstd/xz/bzip2/lz4/brotli stream composition.
- Signature-based format detection.
- Archive browser, selected extraction, destination and conflict policies.
- Background task queue with progress and cancellation.
- Path traversal, link escape, reserved-name, collision, and expansion-limit defenses.
- Interoperability corpus and continuous fuzzing.

Exit criteria: ZIP and TAR-family round trips interoperate with reference tools;
all security fixtures pass; no archive task blocks the UI thread.

## Stage 2 — Beta

Goal: complete the Windows-focused daily workflow.

- 7z read/write and AES encryption.
- Password flow with secret-safe handling.
- Windows file associations, drag-and-drop, taskbar progress, and shell commands.
- Isolated worker process with Windows Job Object limits.
- MSI/MSIX packaging and upgrade tests.
- Performance baselines for throughput, memory, startup, and large listings.

Exit criteria: signed Beta packages install, upgrade, repair, and uninstall on
supported Windows 10/11 environments.

## Stage 3 — Release candidate

Goal: distribution, accessibility, localization, and hardening.

- Simplified Chinese and English UI/docs.
- Keyboard-only, screen-reader, high-contrast, and per-monitor DPI verification.
- Windows Application Certification Kit run.
- WinGet manifest submission.
- Microsoft Store submission and certification.
- SBOM, provenance attestation, checksums, and reproducible-build documentation.

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
