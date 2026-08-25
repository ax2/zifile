---
title: Format support
description: ZiFile's verified and planned archive-format capabilities.
---

Repository round-trip tests cover the capabilities below. Planned operations are not advertised as available in the UI.

| Format | Browse | Extract | Create | Encryption | Status |
| --- | --- | --- | --- | --- | --- |
| ZIP | Yes | Yes | Yes | AES | Implemented |
| 7z | Yes | Yes | Yes | AES | Implemented |
| TAR | Yes | Yes | Yes | No | Implemented |
| TAR + gzip/zstd/xz/bzip2 | Yes | Yes | Yes | No | Implemented |
| Single-stream gzip/zstd/xz/bzip2/lz4/brotli | One entry | Yes | Yes | No | Implemented |
| RAR 1.3–7 | Yes | Yes | No | Read | Beta |

RAR creation is out of scope. Read-only browsing, integrity testing and selective extraction use the pure-Rust `rars` provider (MIT OR Apache-2.0). ZiFile rejects unsafe paths, links and RAR 5+ redirections, applies declared and decoded-size limits, writes to temporary files, and runs archive work in the isolated Worker. Password-protected RAR archives are supported without persisting passwords.
