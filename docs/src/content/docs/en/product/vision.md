---
title: Product vision
description: ZiFile's positioning, user goals, and first-release boundaries.
---

## Positioning

ZiFile is a modern archive and file utility for Windows 10/11. It aims to provide clearer interaction than traditional archive tools, stricter security defaults, and an auditable open-source build chain.

The product is named **ZiFile**, the repository is `ax2/zifile`, the publisher is **ZiCode / zicode.com**, and the planned WinGet ID is `ZiCode.ZiFile`.

## First-release tasks

1. Drop or select an archive and inspect it quickly.
2. Extract all or selected files with clear conflict and risk handling.
3. Select files and folders, then create an archive with a suitable format, compression level, and encryption configuration.
4. Observe background progress and cancel long-running tasks.

## Out of scope for the first release

- Creating RAR archives.
- Building a complete file manager.
- Accounts, cloud sync, advertising, or default telemetry.
- Inventing a new archive format.
- Copying or deriving code from `ouch`, 7-Zip, PeaZip, or similar projects.

## Later extensions

The architecture leaves extension points for checksums, duplicate analysis, batch rename, and file preview. These must not delay the first archive-focused release.
