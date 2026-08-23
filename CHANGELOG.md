# Changelog

All notable changes to ZiFile are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Rust workspace with `zifile-core`, `zifile-cli`, and `zifile-desktop`.
- Shared archive format capability registry and extension detection.
- Conservative default extraction limits.
- Iced desktop technology shell.
- Astro Starlight documentation, roadmap, ADRs, and Stage 0 work log.
- Unit, benchmark, smoke, CI, documentation, and release foundations.
- Real ZIP/ZIP64/AES and 7z/AES create, list, verify, and extraction operations.
- TAR, tar.gz, tar.zst, tar.xz and tar.bz2 archive compositions.
- gzip, Zstandard, XZ, Bzip2, LZ4 and Brotli single-stream operations.
- Signature-based detection and a shared safe extraction policy covering traversal,
  links, Windows device names, case collisions, conflicts and expansion limits.
- CLI archive commands and a modern Iced archive browser/creator with background work.
- Desktop drag-and-drop opens known archives or adds files and folders as creation sources.
- Determinate byte/entry progress, cooperative cancellation, and bounded list-time decoding.
- Deterministic Windows assets, x64/ARM64 MSIX packaging and archive file associations.
- Tag-driven checksums, CycloneDX SBOM, provenance and WinGet 1.12 manifest generation.
- Security-focused fuzz targets and archive throughput benchmarks.
- Bidirectional ZIP and tar.gz interoperability tests against Windows reference tools.
- Simplified Chinese and English desktop UI with system-locale detection and persisted
  language/theme preferences; passwords are never included in settings.
- Archive-path search and bounded 500-row pagination, with a 100,000-entry regression test.
- Desktop shortcuts for opening (`Ctrl+O`), creating (`Ctrl+N`), selecting all (`Ctrl+A`)
  and canceling an active operation (`Escape`).
