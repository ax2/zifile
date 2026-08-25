---
title: Security model
description: Threats, limits, and verification for untrusted archives.
---

Archive contents, names, metadata, links, compression parameters, and password prompts are untrusted. Listing is part of the parser attack surface, not merely extraction.

## Required defenses

- Reject `..`, absolute paths, UNC paths, and destination escape.
- Reject escaping symbolic links, hard links, junctions, and reparse points.
- Reject Windows device names, NTFS alternate data streams, and illegal paths.
- Detect case, Unicode-normalization, and duplicate-entry collisions.
- Enforce entry-count, expanded-size, depth, and compression-ratio limits.
- Never overwrite existing files without an explicit conflict policy.
- Keep passwords out of history, logs, crash reports, and process arguments.

## Boundaries

`SafetyLimits` provides conservative defaults. Limit-aware list/test APIs allow stricter callers, and extraction applies caller limits before creating its destination. Writes use temporary files and atomic replacement.

The UI does not parse archives. IPC limits requests to 16 MiB and events to 4 MiB; entries stream one at a time. The Windows Worker Job allows one process, caps memory at 4 GiB, and kills on close. Passwords travel only over standard input. This is process isolation, not AppContainer permission isolation.

The CLI does not accept a plaintext `--password` argument. Password-bearing commands must explicitly use `--password-stdin` to read one line, keeping secrets out of process arguments. Callers should still pipe from a secure prompt or secret provider instead of writing real passwords as command literals.

The 7z Provider also converts unwindable backend panics from malformed metadata into ordinary errors. OOM, process termination, and sanitizer findings remain Worker-isolation and fuzzing concerns. `cargo-deny` rejects unknown registries, unknown Git sources, wildcard dependencies, and unapproved licenses. RAR requires a separate review.
