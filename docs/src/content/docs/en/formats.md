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
| TAR + gzip/zstd/xz/LZMA/bzip2/LZ4 | Yes | Yes | Yes | No | Implemented |
| Single-stream gzip/zstd/xz/lzma/bzip2/lz4/brotli | One entry | Yes | Yes | No | Implemented |
| RAR 1.3–7 | Yes | Yes | Yes (RAR 5) | Password/header encryption | Beta |
| Windows CAB | Yes | Yes | Yes (MSZIP) | No | Beta |

ZIP reading supports Store, Deflate, Deflate64, BZip2, LZMA, XZ, Zstandard, and PPMd methods, plus AES and legacy ZipCrypto decryption. Creation uses widely compatible Deflate and password-protected creation uses AES-256; legacy ZipCrypto is read-only and is not offered for new encryption. Windows CI verifies Store, Deflate, Deflate64, BZip2, LZMA, XZ, PPMd, AES-256, and ZipCrypto with independent 7-Zip reference archives. Zstandard decoding is independently verified with pinned libarchive ZIPX fixtures, including exact paths, sizes, and per-file hashes.

`.zipx` is recognized as a ZIP reading alias and is included in both desktop open dialogs and the Windows package file association. ZiFile still creates ordinary `.zip` archives by default.

The desktop open dialog also exposes common comic-book, TAR-family, LZMA, Bzip2, and LZ4 aliases such as CBZ/CB7/CBR/CBT, TXZ/TZST/TBZ2, `.tar.lzma`, `.tlz4`, `.lzma`, and `.bz`. `.tar.lzma` uses TAR + LZMA-alone, `.tar.lz4`/`.tlz4` use TAR + LZ4, ordinary `.lzma` uses an LZMA-alone decoder, and `.lz4` uses the single-stream LZ4 decoder. The Windows package registers single-component archive suffixes; Appx manifests reject compound suffixes, so the compound formats remain available in ZiFile's open dialog without being claimed by the MSIX default file association. The package also deliberately does not take over `.epub`; EPUB files can still be selected manually and are inspected as ZIP content.

Legacy ZIP Shrink, Reduce 1–4, and Implode methods are also decoded read-only. Pinned upstream fixtures verify ZiFile's extracted bytes against known content, while 7-Zip independently identifies the archived method. These obsolete algorithms are not offered for new archive creation.

ZIP, 7z, RAR, CAB, and TAR compositions (including TAR + LZMA and TAR + LZ4) accept multiple files and folders when created. RAR creation emits RAR 5 archives at levels 0–5 and can encrypt headers with a password; RAR cannot be updated or renamed in place and cannot preserve empty directories. CAB creation uses fixed MSZIP compression and rejects sources containing empty directories because CAB cannot represent them. Single-stream gzip, Zstandard, XZ, LZMA, Bzip2, LZ4, and Brotli require exactly one existing file; use the corresponding TAR composition for folders or multiple items. The desktop validates this before opening the destination dialog.

When extracting a single-stream format, legacy aliases such as `.lzma` and `.bz` preserve the original filename stem instead of gaining an unnecessary `.out` suffix because their canonical extensions differ.

The creation UI derives its compression-level range from the selected encoder: ZIP, 7z, gzip, XZ, and LZMA use 0–9; RAR uses 0–5; Zstandard uses 0–22; Bzip2 uses 1–9; and Brotli uses 0–11. Plain TAR, CAB's fixed MSZIP encoder, and the current LZ4 encoder have fixed settings, so those formats do not expose an inert level control. The CLI exposes the same ranges through `zifile formats`; `create --level` rejects out-of-range values, and an explicit level is also rejected for TAR/CAB/LZ4 so input is neither silently clamped nor ignored. 7z writes the selected level into its LZMA2 parameters, and RAR writes it into its RAR 5 encoder, preserving the selection when encryption is enabled.

RAR 1.3–7 browsing, integrity testing and selective extraction, plus RAR 5 creation, use the pure-Rust `rars` provider (MIT OR Apache-2.0). Creation supports levels 0–5 and optional password-protected headers. ZiFile rejects unsafe paths, links and RAR 5+ redirections, applies declared and decoded-size limits, writes to temporary files, and runs archive work in the isolated Worker. Passwords are never persisted. RAR volumes, recovery records, updates and renames remain outside the current creation contract.

CAB uses the pure-Rust, MIT-licensed `cab` provider. Creation emits fixed MSZIP cabinets without encryption; browsing, integrity testing, and selective safe extraction cover None, MSZIP, and LZX content. Quantum compression and multi-cabinet sets are not supported; multi-cabinet headers are explicitly rejected before browsing. CAB containers are not safely updated or renamed after creation.
