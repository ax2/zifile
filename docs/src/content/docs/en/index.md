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
    - text: GitHub
      link: https://github.com/ax2/zifile
      icon: external
      variant: minimal
---

ZiFile is an MIT-licensed project by ZiCode. Its first release focuses on safely browsing, creating, and extracting common archive formats, with room to grow into a trusted set of file operations.

The project is currently in **Stage 1 — Alpha development**. ZIP, 7z, TAR compositions, and the main single-stream formats support real creation, browsing, integrity testing, and safe extraction. The desktop UI and CLI share the same Rust core. Cancellation, the isolated Worker, Windows integration, signed installation, and store certification continue to be hardened.

## Design principles

- **Safe defaults:** every archive is treated as untrusted input.
- **Honest capabilities:** the UI exposes only operations declared by the backend.
- **Background execution:** parsing, compression, and extraction must not block the UI.
- **Traceable releases:** versions, builds, documentation, SBOMs, and release records stay aligned.
- **Windows first:** installation, Shell integration, and accessibility on Windows 10/11 come first.
