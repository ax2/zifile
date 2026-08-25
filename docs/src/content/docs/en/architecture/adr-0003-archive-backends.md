---
title: "ADR-0003: Archive backend composition"
description: ZiFile's archive implementations, license boundary, and replacement policy.
---

- Status: accepted
- Date: 2026-08-24

## Decision

ZiFile uses replaceable pure-Rust or Rust-native backends: `zip` for ZIP, `sevenz-rust2` for 7z, `rars` for read-only RAR, `tar`, `flate2` for gzip, `zstd`, statically linked `xz2`, `bzip2`, `lz4_flex`, and `brotli`.

Only `zifile-core` exposes these backends. The core centrally enforces path normalization, link rejection, conflict handling, resource limits, cancellation, and temporary-file writes.

## Constraints and outcome

- New or replaced backends require license, provenance, hostile-corpus, interoperability, fuzz, and performance checks.
- The capability matrix reports only tested implementations.
- RAR creation is out of scope. Read-only RAR 1.3–7 support uses `rars` 0.9.3 (MIT OR Apache-2.0) after license and source review; its beta capability stays behind core safety checks, Worker isolation, fuzzing, hostile fixtures, and reference-reader interoperability.
- C/C++ backends require a separate supply-chain review and are considered only when Rust options cannot meet compatibility needs.

This combination covers the main open formats used on Windows while keeping a clear MIT distribution boundary. Metadata capabilities may differ between formats.
