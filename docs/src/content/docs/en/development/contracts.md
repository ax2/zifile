---
title: Public contracts and version policy
description: Compatibility boundaries for the ZiFile 1.0 CLI, core providers, IPC, and release version.
---

This page defines the 1.0 candidate contract. It may still change with release notes and migration guidance before 1.0. After 1.0, semantic versioning applies and breaking changes require a new major version.

## CLI contract

The 1.0 candidate keeps these commands: `formats`, `detect`, `list`, `test`, `extract`, and `create`. Creation format values are `zip`, `seven-zip`, `tar`, `tar-gzip`, `tar-zstd`, `tar-xz`, `tar-lzma`, `tar-bzip2`, `gzip`, `zstandard`, `xz`, `lzma`, `bzip2`, `lz4`, and `brotli`. Conflict values are `overwrite`, `skip`, `rename`, and `error`.

Password input is available only through explicit `--password-stdin`. The CLI does not accept a plaintext password argument or promise an interactive prompt.

| Exit code | Meaning |
| --- | --- |
| `0` | The operation succeeded |
| `1` | A file, format, password, policy, backend, or other runtime error occurred |
| `2` | Clap rejected command-line syntax or an argument |

Runtime errors go to standard error with an `error: ` prefix. Ordinary success prose is human-facing and may improve without changing command semantics; automation must not parse those sentences. `zifile formats` is the stable tab-separated capability table and includes `CREATE_INPUT` (`files-or-directories`, `single-file`, or `none`) and `COMPRESSION_LEVEL` (an inclusive range, `fixed`, or `none`). Adjustable formats default to level 6 when `create --level` is omitted; an explicit value is validated against the resolved format, and an out-of-range value is a runtime input error with exit code `1`. A `fixed` format requires `--level` to be omitted. The CLI neither silently clamps nor ignores explicit input.

## Core provider contract

Desktop, CLI, and Worker share `zifile-core`. The 1.0 candidate boundary includes:

- `ArchiveFormat`, `FormatCapabilities`, `CreateInputKind`, and `ReleaseStage`;
- detection, list, test, create, and extract entry points;
- `CreateOptions`, `ExtractOptions`, `ConflictPolicy`, `SafetyLimits`, cancellation, and progress types;
- `ZiFileError` and `ZiFileResult`.

Adding a format, capability, or optional setting is a compatible extension. Removing or renaming a public format, changing an existing option's default safety semantics, weakening safety limits, or reinterpreting an existing error requires major-version review. RAR creation is not part of the 1.0 contract.

The Worker JSON Lines IPC has an independent `PROTOCOL_VERSION`. Incompatible clients and Workers must reject each other explicitly instead of guessing from fields.

## Single version source

`[workspace.package].version` in `Cargo.toml` is the only product-version source. The documentation package, all six workspace packages, internal dependency pins, and `Cargo.lock` must agree. A release tag must be exactly `v<workspace-version>`. The four-part MSIX version is derived deterministically from the same value—for example, `0.1.0-alpha.1` becomes `0.1.0.1`.

`scripts/Test-VersionConsistency.ps1` runs in normal CI and the Release workflow. Manual Release validation no longer accepts a mutable version input and always builds the current workspace version.
