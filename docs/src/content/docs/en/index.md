---
title: ZiFile
description: A modern, safe, open-source archive and file utility for Windows.
template: splash
hero:
  tagline: A modern Windows archive and file utility built from scratch in Rust
  actions:
    - text: View the roadmap
      link: product/roadmap/
      icon: right-arrow
    - text: Getting started
      link: guides/getting-started/
      icon: right-arrow
    - text: GitHub
      link: https://github.com/ax2/zifile
      icon: external
      variant: minimal
---

ZiFile is an MIT-licensed project by ZiCode. Its first release focuses on safely browsing, creating, and extracting common archive formats, with room to grow into a trusted set of file operations.

The project is currently in **Stage 3 — Release candidate preparation**. ZIP, 7z, TAR compositions, and the main single-stream formats support real creation, browsing, integrity testing, and safe extraction; RAR 1.3–7 and Windows CAB are available as read-only beta providers. The desktop UI and CLI share the same Rust core, while x64/ARM64 builds, package audits, reproducibility checks, and release rehearsals are automated. Trusted signing, real foreground validation, physical ARM64, WACK, Partner Center, Store, and WinGet remain 1.0 external gates.

Start with [Getting started](/zifile/en/guides/getting-started/). See [Troubleshooting](/zifile/en/guides/troubleshooting/) for format, safety-policy, Worker, or development-package failures.

## Design principles

- **Safe defaults:** every archive is treated as untrusted input.
- **Honest capabilities:** the UI exposes only operations declared by the backend.
- **Background execution:** parsing, compression, and extraction must not block the UI.
- **Traceable releases:** versions, builds, documentation, SBOMs, and release records stay aligned.
- **Windows first:** installation, Shell integration, and accessibility on Windows 10/11 come first.
