---
title: "ADR-0007: Safe updates to existing archives"
description: Boundaries for adding, merging, rebuilding, and atomically replacing multi-entry archives.
---

## Status

Accepted on 2026-08-31. The 0.1.9 candidate wires the capability through the core API, CLI, Worker, and both desktop UIs.

## Background

Users need to add files or folders from the archive browser and remove selected archive entries. The current ZIP and 7z provider boundaries do not expose a reusable in-place append or removal abstraction, and TAR compositions must rewrite their outer stream. Overwriting the original directly could leave an unusable archive after an encoder error, cancellation, or disk failure.

## Decision

- Expose updates only for ZIP, 7z, and TAR-family multi-entry containers. gzip, Zstandard, XZ, LZMA, Bzip2, LZ4, and Brotli are single-file streams; RAR and CAB can be created but do not support update or rename.
- The core fully lists and safely extracts the original into a sibling staging workspace, then merges additions by source root or removes selected archive-relative paths and their descendants. Colliding regular files are replaced; file/directory type collisions, links, and reparse points are rejected.
- The rebuild reuses the same safety limits, password, cancellation token, and progress channel. Any failure or cancellation leaves the original archive untouched.
- Commit uses platform atomic replacement semantics: Windows calls `MoveFileExW` with replace/write-through flags, while other platforms use `rename`. RAII removes the staging workspace; it is never a release asset.
- The UI exposes update actions only when `ArchiveFormat::supports_update()` allows them. CLI and Worker return an explicit `UnsupportedOperation` for unsupported formats instead of silently selecting another encoder.

## Trade-offs and follow-up

Full extraction and rebuilding costs more time and temporary disk space than a provider-specific incremental editor, but it keeps the boundary uniform, testable, and failure-safe. A future incremental provider must preserve the same atomic commit, limits, cancellation, and link-rejection contracts and add independent interoperability evidence.
