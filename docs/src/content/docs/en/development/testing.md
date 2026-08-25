---
title: Testing strategy
description: Unit, property, interoperability, security, performance, fuzz, and smoke gates.
---

## Pull-request gates

- Rust format, strict all-target/all-feature Clippy, and locked all-target tests.
- License, provenance, and advisory policy.
- Starlight type checking and static build.
- Criterion and libFuzzer target compilation.
- Real Worker and packaging-policy smoke tests.
- Windows ZIP, tar.gz, and 7z bidirectional interoperability.

## Layers

Unit and property tests cover detection, paths, limits, conflict policy, randomized trees, and boundaries. Security corpora cover traversal, bombs, links, collisions, corruption, and truncation. Interoperability compares ZiFile in both directions with Windows reference tools. Performance tracks throughput, ratio, peak memory, startup, and large lists. Smoke tests cover CLI, desktop, Worker IPC/cancellation, packaging, installation, associations, and uninstall as their environments become available.

The Worker smoke streams a real list request and requires metadata, a Unicode entry, and exactly one terminal event. It then cancels a 32 MiB random 7z creation and requires timely exit with no target or temporary residue. Queue unit tests cover strict FIFO, 32-item capacity, stale completion IDs, clearing, and immediate sensitive-payload release.

CLI password tests cover explicit opt-in, CRLF/LF removal, preservation of surrounding spaces, and rejection of missing or empty input. The foundation smoke requires help to expose only `--password-stdin`, then creates, tests, and extracts a real AES 7z through standard input without printing the fixed test password.

Every CI compiles fuzz targets. Weekly bounded campaigns exercise path policy, format detection, and every supported parser for 180 seconds each. Two historical malformed 7z artifacts (292 and 173 bytes) are replayed at every parser campaign start. Their discoveries led to Rust 1.93.0 and bounded-metadata `sevenz-rust2` 0.22.0; targeted run `32813469578` replayed both, executed another 498,937 inputs in 181 seconds, peaked at 370 MiB RSS, and found no new crash.

The 100,000-entry UI model constructs at most 500 visible rows. A real deterministic ZIP baseline validates Worker listing, search, paging, 50% scrolling, tree-wide memory sampling, and cancellation with Worker reclamation. Five cancellation runs completed at 930.78 ms median and 1088.73 ms p95 with zero Workers remaining. These are same-machine regression baselines, not universal performance promises.

Foreground keyboard automation checks internal WebView2 focus, bilingual forward/reverse navigation, disabled-control skipping, 7z selection, level adjustment, password clearing, and source buttons. It verifies the exact ZiFile foreground window before every key and never records the password. Full archive/extract traversal and assistive-technology certification remain open.

Reproducibility separately performs clean x64/ARM64 double builds. Schema-v2 evidence traced the former 4/5 result to `build-a`/`build-b` target paths embedded by generated `glutin_wgl_sys` code in the default Iced executable. The script remaps both isolated roots to one virtual path; run `32826187552` then proved 5/5 and `reproducible=true` on both architectures.
