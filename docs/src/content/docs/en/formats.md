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
| RAR | No | No | No | To be assessed | Pending review |

RAR creation is out of scope. Read-only RAR support requires licensing and security review and must not alter the MIT licensing boundary of the main project.
