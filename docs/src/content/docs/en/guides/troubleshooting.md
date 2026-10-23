---
title: Troubleshooting
description: Common ZiFile open, extraction, creation, Worker, and installation failures.
---

## An archive will not open or test

- Confirm that the download completed and compare its SHA-256 with a trusted source when one is available.
- The extension may not match the real format. ZiFile fails safely instead of guessing the wrong decoder.
- Encrypted archives require the correct password. The CLI uses `--password-stdin`; an empty password is rejected.
- If a file was still being written or a transient I/O fault occurred, the failed-open view retains its path and offers one **Reload** attempt; an encrypted file list instead presents password entry and **Unlock**. Do not repeatedly retry an untrusted archive or one rejected by a safety limit.
- RAR volumes, recovery records, or uncommon encodings may be outside the current scope. RAR reading is Beta; RAR creation is outside ZiFile's current contract.

For a compatibility report, provide the creating tool and version, format options, ZiFile version, and a public minimal reproducer when possible. Never upload customer data, passwords, or private archives.

## Extraction is rejected by safety policy

Archives containing `..`, absolute paths, device names, case collisions, links, or abnormal expansion are rejected. This often signals corruption, incompatibility, or hostile content. ZiFile has no switch to disable these safety boundaries. A trusted tool may be used to inspect structure in a controlled environment, but never overwrite an important destination.

ZiFile also rejects an extraction root or existing parent directory that is a symbolic link, junction, or reparse point. Choose a normal directory so output cannot be redirected outside the selected destination.

## The Create button is unavailable

- A gzip, zstd, xz, lzma, bzip2, lz4, or Brotli stream requires exactly one existing file and cannot accept a directory.
- ZIP, 7z, and TAR compositions accept multiple files and folders.
- Creation is blocked before the save dialog when sources are empty, no longer exist, are symbolic links, or the output is invalid.
- RAR can be browsed, tested, and extracted; RAR creation, update, and rename are not supported.

After the first source is added, the create page chooses its parent directory automatically; a folder source uses the folder's parent. Use **Choose another folder** when the archive should be written elsewhere. Format and compression-level changes are saved as defaults for later creates and Explorer quick create.

## Explorer quick create does not work

After a formal package installs and activates the Shell extension, Explorer exposes both the full create-page command and the direct quick-create command for files, folders, and folder backgrounds. Quick create uses the saved format and compression level without opening the main window. Selected items are archived beside the first item; a folder-background invocation archives the current directory's immediate children into that directory.

If quick create reports an error, confirm that the saved format accepts the source shape. gzip, zstd, xz, lzma, bzip2, lz4, and Brotli accept one existing file only; use ZIP, 7z, or a TAR composition for folders. Errors are shown in a short dialog and do not open the full create page.

## An operation is slow or cancelled

Large archives are protected by entry-count, expanded-size, compression-ratio, and memory limits; lists are paged in groups of 500. Cancellation waits for a Worker cancellation point and terminates the Worker after a timeout. Save the error text and reopen the application after a failure rather than repeatedly retrying the same suspicious archive.

## The Worker exits unexpectedly

ZiFile converts Worker crashes, protocol failures, and resource-limit exits into operation errors while keeping the desktop process available. Reopening the archive starts a new Worker. For a stable reproducer, record the app version, operation, format, file size, and exact error, then follow the safe reporting rules for a minimal sample.

## A development package will not install

An unsigned `.Dev` MSIX is not a production artifact, and some Windows versions reject its Publisher. Do not import a test root, disable SmartScreen, or weaken system trust policy. Use the complete runnable directory for development verification and wait for a publicly trusted or Microsoft Store package for production installation.

The Explorer menu can appear only after a formal package installs and activates the Shell extension. The current Release Candidate does not treat registration markup as proof of trusted installation or lifecycle behavior.

## Report safely

Use [GitHub Issues](https://github.com/ax2/zifile/issues) for ordinary defects. Follow the [security policy](https://github.com/ax2/zifile/security/policy) for suspected vulnerabilities, and never publish unpatched details, malicious archives, credentials, or private data in a public issue. Redact local usernames and paths from logs when needed.
