---
title: Format support
description: ZiFile's verified and planned archive-format capabilities.
---

Repository integration tests cover the capabilities below. Planned operations are not advertised as available in the UI.

| Format | Browse | Extract | Create | Encryption | Status |
| --- | --- | --- | --- | --- | --- |
| ZIP | Yes | Yes | Yes | AES | Implemented |
| 7z | Yes | Yes | Yes | AES | Implemented |
| TAR | Yes | Yes | Yes | No | Implemented |
| TAR + gzip/zstd/xz/bzip2 | Yes | Yes | Yes | No | Implemented |
| Single-stream gzip/zstd/xz/bzip2/lz4/brotli | One entry | Yes | Yes | No | Implemented |
| RAR 1.3–7 | Yes | Yes | No | Read | Beta |
| Windows CAB | Yes | Yes | No | No | Beta |

ZIP reading supports Store, Deflate, Deflate64, BZip2, LZMA, XZ, Zstandard, and PPMd methods, plus AES and legacy ZipCrypto decryption. Creation uses widely compatible Deflate and password-protected creation uses AES-256; legacy ZipCrypto is read-only and is not offered for new encryption. Windows CI verifies Store, Deflate, Deflate64, BZip2, LZMA, XZ, PPMd, AES-256, and ZipCrypto with independent 7-Zip reference archives. ZIP Zstandard decoding is currently covered by the locked backend tests and is not claimed as 7-Zip corpus evidence.

Legacy ZIP Shrink, Reduce 1–4, and Implode methods are also decoded read-only. Pinned upstream fixtures verify ZiFile's extracted bytes against known content, while 7-Zip independently identifies the archived method. These obsolete algorithms are not offered for new archive creation.

ZIP, 7z, and TAR compositions accept multiple files and folders when created. Single-stream gzip, Zstandard, XZ, Bzip2, LZ4, and Brotli require exactly one existing file; use the corresponding TAR composition for folders or multiple items. The desktop validates this before opening the destination dialog.

RAR creation is out of scope. Read-only browsing, integrity testing and selective extraction use the pure-Rust `rars` provider (MIT OR Apache-2.0). ZiFile rejects unsafe paths, links and RAR 5+ redirections, applies declared and decoded-size limits, writes to temporary files, and runs archive work in the isolated Worker. Password-protected RAR archives are supported without persisting passwords.

CAB uses the pure-Rust, MIT-licensed `cab` provider. Browsing, integrity testing, and selective safe extraction currently cover None, MSZIP, and LZX content. Quantum compression and multi-cabinet sets are not supported; multi-cabinet headers are explicitly rejected before browsing, and CAB creation is not exposed.
