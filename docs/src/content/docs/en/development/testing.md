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
- Official 7-Zip codec, filter, solid-mode, and AES interoperability with JSON evidence.
- Pinned RAR 1.3/1.5/3/5/7, PPMd, filter, encrypted-header and unsafe-link corpus cross-checked against 7-Zip with JSON evidence.

## Layers

Unit and property tests cover detection, paths, limits, conflict policy, randomized trees, and boundaries. Security corpora cover traversal, bombs, links, collisions, corruption, and truncation. Interoperability compares ZiFile in both directions with Windows reference tools. Performance tracks throughput, ratio, peak memory, startup, and large lists. Smoke tests cover CLI, desktop, Worker IPC/cancellation, packaging, installation, associations, and uninstall as their environments become available.

The Worker smoke streams a real list request and requires metadata, a Unicode entry, and exactly one terminal event. It then cancels a 32 MiB random 7z creation and requires timely exit with no target or temporary residue. Queue unit tests cover strict FIFO, 32-item capacity, stale completion IDs, clearing, and immediate sensitive-payload release.

CLI password tests cover explicit opt-in, CRLF/LF removal, preservation of surrounding spaces, and rejection of missing or empty input. The foundation smoke requires help to expose only `--password-stdin`, then creates, tests, and extracts a real AES 7z through standard input without printing the fixed test password.

`tests/smoke/packaging-policy.ps1` dynamically parses the current sixteen release, corpus, and repository-policy PowerShell scripts. It rejects missing/partial Partner Center identity, malformed Name/X.500 Publisher, missing cloud inputs, unsupported providers, development identities, unsigned OID publishers, invalid signed artifacts, and incomplete 1.0 readiness; it accepts valid input and fully evidenced 11/11 fixtures. It also requires post-signing audit, signed-only publishing, least privilege, signing timeout/concurrency, the rotation/emergency-stop/revocation runbook, and version, release-note, contributor, security, and readiness gates in CI. Policy smoke cannot replace a real account, cloud-HSM signature, or x64/ARM64 package-content audit.

The official 7-Zip corpus gate uses `7z.exe` on the GitHub Windows Runner. Reference-created cases cover Copy, LZMA, LZMA2+BCJ, Deflate, BZip2, PPMd, and LZMA2+AES with encrypted headers. In the reverse direction, 7-Zip must test and extract both ordinary and AES archives created by ZiFile. Every case compares the complete relative file set and SHA-256 content hashes; the uploaded JSON evidence contains no password. CI `32836336921` passed all nine cases with 7-Zip 26.02; the evidence JSON SHA-256 is `06278BB8B96AB683A3C117BA5E30F1B4AB1CF89F1BBF01E72BAC0CC26B49DB14`.

The RAR gate downloads six fixtures from the pinned `rars` source commit `7d8f9386ef777a2415da34fe1db193d8471ff7d0`, verifies hard-coded SHA-256 values before use, and compares extraction trees byte for byte. It covers RAR 1.3, 1.54 multi-file, RAR 3 PPMd, RAR 5 compression and E8E9 filtering, plus a WinRAR 7.21 encrypted-header/Quick Open archive. Three pinned link/redirection archives must be rejected without output. CI `32853686537` passed all six valid and three rejection cases; the evidence JSON SHA-256 is `4C52D0240B911609C7DDB0CACB2E484F56C8F886E216347603B228261C4EE8EF`. Because current 7-Zip no longer reads RAR 1.3, that case is compared with the known-good extracted tree from the same pinned upstream commit; the other five valid archives remain cross-checked against 7-Zip 26.02.

Every CI compiles fuzz targets. Weekly bounded campaigns exercise path policy, format detection, and every supported parser for 180 seconds each. Two historical malformed 7z artifacts (292 and 173 bytes) are replayed at every parser campaign start. Their discoveries led to Rust 1.93.0 and bounded-metadata `sevenz-rust2` 0.22.0; targeted run `32813469578` replayed both, executed another 498,937 inputs in 181 seconds, peaked at 370 MiB RSS, and found no new crash.

The RAR verification benchmark uses a deterministic 8 MiB RAR 5 method-3 archive with low-frequency pseudorandom noise, retaining compression work without exceeding the default 1000:1 expansion guard. The initial local Windows x64 baseline measured 58.12–64.49 ms, or 124.06–137.65 MiB/s. This is a same-machine regression baseline, not a universal performance claim. The original highly periodic fixture was correctly rejected as exceeding the safety ratio and was not used to bypass that guard.

The 100,000-entry UI model constructs at most 500 visible rows. A real deterministic ZIP baseline validates Worker listing, search, paging, 50% scrolling, tree-wide memory sampling, and cancellation with Worker reclamation. Five cancellation runs completed at 930.78 ms median and 1088.73 ms p95 with zero Workers remaining. These are same-machine regression baselines, not universal performance promises.

`tests/smoke/store-listing.ps1` verifies that the Simplified Chinese and English Store JSON satisfies Partner Center limits for descriptions, short descriptions, features, keywords, system requirements, licensing, and HTTPS URLs. It also requires each readable listing page to contain every authoritative JSON description paragraph and feature verbatim. Negative fixtures prove that an oversized feature, excess keywords, and a URL inside the description are rejected. This gate covers copy, not screenshots, age ratings, official identity, or certification.

The same smoke test exercises atomic screenshot import: it generates eight valid PNGs, requires complete capture metadata, imports from an independent directory, and reruns the formal manifest validator. Missing metadata, undersized images, duplicate content, and attempts to overwrite existing assets all fail. Temporary images are removed and never enter the formal asset directory.

`tests/helpers/msix-repair` is a C# test-only console helper; the product remains Rust-first. CI compiles it against a locked Windows App SDK 1.8 dependency, then a PowerShell supervisor that does not load the App SDK launches the non-mutating `--probe`. Even if App SDK initialization blocks before the helper entry point, the supervisor terminates the process directly after 15 seconds; the workflow adds a two-minute outer bound. A Runner that does not return records an incomplete/unsupported probe instead of hanging or claiming Repair passed, and a one-second blocking fixture continuously proves this hard-timeout path. When Repair is supported, the trusted lifecycle writes a random package LocalState sentinel, requires `RepairPackageAsync` to preserve it, then requires `Reset-AppxPackage` to remove it. Unsupported systems record `unsupported` explicitly.

`tests/smoke/wack-readiness.ps1` uses an unsigned development-package fixture to prove readiness reports a missing WACK tool, invalid signatures, development identity, unsigned publisher, wrong minimum OS, and package/audit hash mismatch. It also proves `-RequireReady` persists structured failure evidence. This smoke does not run WACK or replace a formal signed-candidate certification report.

Foreground keyboard automation checks internal WebView2 focus, bilingual forward/reverse navigation, disabled-control skipping, 7z selection, level adjustment, password clearing, and source buttons. It verifies the exact ZiFile foreground window before every key and never records the password. Full archive/extract traversal and assistive-technology certification remain open.

Reproducibility separately performs clean x64/ARM64 double builds. Schema-v2 evidence traced the former 4/5 result to `build-a`/`build-b` target paths embedded by generated `glutin_wgl_sys` code in the default Iced executable. The script remaps both isolated roots to one virtual path; run `32826187552` then proved 5/5 and `reproducible=true` on both architectures.
