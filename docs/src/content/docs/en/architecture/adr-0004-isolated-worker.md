---
title: "ADR-0004: Isolated archive Worker"
description: Process boundaries, IPC, and Windows Job Object decisions for archive operations.
---

## Status

Accepted on 2026-08-24.

## Context and decision

Archive parsing handles untrusted input. A thread pool protects responsiveness but cannot protect the UI process from parser crashes, runaway memory, or failed cancellation.

- The `zifile-worker` runtime performs desktop list, test, extract, and create operations through `zifile-core`. The portable desktop EXE starts that runtime in-process via `--zifile-worker`; the standalone `zifile-worker.exe` remains a compatibility entry point for MSIX and development environments.
- `zifile-worker-protocol` defines versioned JSON Lines messages. Entries stream individually and exactly one terminal event is required.
- Requests travel over standard input, so passwords never enter command lines. Requests are capped at 16 MiB, events at 4 MiB, and captured standard error at 64 KiB.
- On Windows, the Worker enters a Job Object before receiving a request. It permits one active process, caps process memory at 4 GiB, and uses kill-on-close.
- Versioned control messages cooperatively cancel create and extract, including 7z reads. Forced termination follows only if the Worker remains alive after two seconds.

## Consequences

Parser failures normally terminate only the Worker. Runnable directories and MSIX packages retain an architecture-matching Worker entry, while GitHub Release portable EXEs embed the runtime and do not require a second file. A Job Object is not a permission sandbox: the Worker retains the current user's file access. AppContainer and finer brokering remain possible defense-in-depth work.
