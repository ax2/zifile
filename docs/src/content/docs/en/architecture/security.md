---
title: Security model
description: Threats, limits, and verification for untrusted archives.
---

Archive contents, names, metadata, links, compression parameters, and password prompts are untrusted. Listing is part of the parser attack surface, not merely extraction.

## Required defenses

- Reject `..`, absolute paths, UNC paths, and destination escape.
- Reject escaping symbolic links, hard links, junctions, and reparse points.
- Reject symbolic links, junctions, and reparse points in the extraction root and existing output parents, so writes cannot follow a host link outside the selected destination.
- Reject Windows device names, NTFS alternate data streams, and illegal paths.
- Detect case, Unicode-normalization, and duplicate-entry collisions.
- Enforce entry-count, expanded-size, depth, and compression-ratio limits.
- Never overwrite existing files without an explicit conflict policy.
- Keep passwords out of history, logs, crash reports, and process arguments.

## Boundaries

`SafetyLimits` provides conservative defaults. Limit-aware list/test APIs allow stricter callers, and extraction applies caller limits before creating its destination. Writes use temporary files and atomic replacement.

TAR parsing accumulates declared entry sizes immediately after each header and checks expanded-size and ratio limits before skipping compressed payloads. This applies to TAR, TAR+gzip, TAR+Zstandard, TAR+XZ, TAR+LZMA, and TAR+Bzip2, so listing does not decode over-budget data merely to reach the next header.

The UI does not parse archives. IPC limits requests to 16 MiB and events to 4 MiB; entries stream one at a time. The Windows Worker Job allows one process, caps memory at 4 GiB, and kills on close. Passwords travel only over standard input. This is process isolation, not AppContainer permission isolation.

The CLI does not accept a plaintext `--password` argument. Password-bearing commands must explicitly use `--password-stdin` to read one line, keeping secrets out of process arguments. Callers should still pipe from a secure prompt or secret provider instead of writing real passwords as command literals.

The 7z and RAR Providers convert unwindable backend panics from malformed metadata into ordinary errors. RAR additionally rejects Unix links, Windows reparse entries and RAR 5+ redirections before decoding; decoded bytes are counted independently of declared sizes and files remain temporary until the complete operation succeeds. OOM, process termination, and sanitizer findings remain Worker-isolation and fuzzing concerns. `cargo-deny` rejects unknown registries, unknown Git sources, wildcard dependencies, and unapproved licenses.

## Reporting and support scope

GitHub private vulnerability reporting is not currently enabled. Until it is enabled, send reports privately to `ax2@zicode.com` with the subject `ZiFile security report`. Do not attach an unpatched vulnerability, working exploit, malicious archive, password, credential, or customer data to a public issue. Include the affected version or commit, impact, reproduction steps, and a minimal non-sensitive test case when practical.

Before the first stable release, only the default branch receives security fixes, and the project does not promise a fixed response SLA. Parser and path escape defects, links or reparse points, resource exhaustion, password exposure, privilege boundaries, package integrity, and shell integration are in scope. The root [`SECURITY.md`](https://github.com/ax2/zifile/blob/main/SECURITY.md) is authoritative for disclosure and future supported-version information.
