---
title: Architecture overview
description: ZiFile's layers, crate boundaries, and task-execution model.
---

```text
Iced UI / Dioxus accessibility candidate -- versioned IPC -- isolated Worker -- zifile-core -- Provider
CLI ------------------------------------------------------------------------┘
                                                                                 |
                                                                         Windows file system
```

## Core and Providers

`zifile-core` defines capabilities, requests, progress, conflict policies, safety limits, and unified errors without depending on a UI or Windows API. Providers explicitly declare operations such as list, extract, create, test, and encryption. The UI renders declared capabilities instead of guessing from a format name.

## Desktop and scheduler

Iced is the current baseline; Dioxus/WebView2 validates a semantic Windows accessibility path. Both use the same Worker IPC and a 32-operation in-memory FIFO scheduler. Open, reload, test, extract, and create requests are snapshotted and run in order. Monotonic IDs prevent stale completion events from advancing the queue. Clearing pending work does not cancel the active Worker, and queued paths or passwords are never persisted.

## Worker boundary

All desktop archive operations use versioned JSON Lines messages with streamed entries. The Windows client assigns the Worker to a one-process, 4 GiB, kill-on-close Job Object. Create and extract cancel cooperatively before a two-second forced cleanup. This limits crash and memory impact but does not remove the current user's filesystem permissions.

## Compatibility

Targets are Windows 10/11 x64 and ARM64. Windows 11 may use modern backdrop effects while Windows 10 receives an opaque fallback. Core archive crates stay independent of Windows for testing and future reuse.
