---
title: Getting started
description: Browse, test, extract, and create archives with ZiFile.
---

ZiFile is currently a Stage 4 public release and has not shipped through Microsoft Store or WinGet. GitHub users can obtain the unsigned Windows build from the current [v0.1.7 Release](https://github.com/ax2/zifile/releases/tag/v0.1.7); verify every download with the included `SHA256SUMS.txt` first. For installation, choose the single all-in-one `ZiFile-0.1.7.0-windows.msixbundle`, which contains x64 and ARM64 packages. The portable downloads are `zifile-windows-x64.exe` and `zifile-windows-arm64.exe`; each is a self-contained standalone program and does not need a separate Worker or DLL. Never import an unknown root certificate or disable Windows security checks to install a development package.

## Open and inspect an archive

1. Start ZiFile and choose **Open archive**, or press `Ctrl+O`.
2. Select a ZIP, 7z, RAR, CAB, TAR, or supported compression stream. A known archive can also be dropped onto the window.
3. Filter paths with search. Large archives are displayed in bounded pages of 500 rows.
4. Run an integrity test before extraction when appropriate. If a 7z or RAR archive also encrypts its file list, the first failure retains the selected file and presents a password retry view. Passwords are never stored in settings and are cleared before another archive is opened.

Detection uses both signatures and extensions. If an extension disagrees with the content, ZiFile reports a detection or parsing error instead of forcing the wrong decoder.

## Extract safely

Select all or individual entries, choose a destination and conflict policy, then start extraction. ZiFile rejects traversal, absolute paths, Windows device names, case collisions, unsafe links, extraction destinations containing symbolic links, junctions, or reparse points, and content beyond entry-count, expanded-size, or compression-ratio limits. Treat such a rejection as a reason to verify the archive's source, not as a prompt to disable safety boundaries.

Work runs in an isolated Worker. Pressing `Escape` or choosing **Cancel** requests cooperative cancellation and terminates the Worker when necessary. Files already completed are not presented as if extraction had transactional rollback.

## Create an archive

1. Choose **Create archive** or press `Ctrl+N`.
2. Add files or folders, or drop sources onto the window.
3. Choose a format, compression level, optional password, and output path.

ZIP, 7z, and TAR compositions accept multiple files and folders. gzip, Zstandard, XZ, LZMA, Bzip2, LZ4, and Brotli are single-file streams and require exactly one existing file; use the matching TAR composition for a directory. RAR and CAB are read-only and cannot be created.

## Queue and settings

Open, test, extract, or create requests can be submitted while another operation runs. ZiFile keeps at most 32 operations and executes them in order. **Clear queue** removes only waiting work; **Cancel** affects only the active operation. Language and light/dark theme are stored in `%LOCALAPPDATA%\ZiFile\settings.conf`; paths, recent history, and passwords are not stored.

The **About** page identifies the running version, MIT license, format-family count, project address, and local-processing privacy boundary; press `F1` to open it directly. It can also open the project home, English documentation, and English privacy policy in the default browser; the footer reports an error if Windows cannot launch a link. Use this version when reporting a problem rather than relying only on the installer filename.

## Command line

```powershell
zifile formats
zifile list archive.zip
zifile test archive.7z
zifile extract archive.zip output --conflict rename
zifile create output.7z files --format seven-zip --level 9
```

The `COMPRESSION_LEVEL` column in `zifile formats` gives each format's inclusive range; `fixed` means the encoder has no adjustable level and `--level` must be omitted. Adjustable formats default to level 6 when the option is omitted; an out-of-range value is reported instead of continuing with a different level.

Encrypted operations read a password from standard input. ZiFile does not accept a plaintext password argument that would enter process arguments or ordinary shell history:

```powershell
$password | zifile test archive.7z --password-stdin
$password | zifile extract archive.7z output --password-stdin
```

See [Troubleshooting](/zifile/en/guides/troubleshooting/) for common failures and [Format support](/zifile/en/formats/) for the capability matrix.
