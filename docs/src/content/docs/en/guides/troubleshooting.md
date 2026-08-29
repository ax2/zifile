---
title: Troubleshooting
description: Common ZiFile open, extraction, creation, Worker, and installation failures.
---

## An archive will not open or test

- Confirm that the download completed and compare its SHA-256 with a trusted source when one is available.
- The extension may not match the real format. ZiFile fails safely instead of guessing the wrong decoder.
- Encrypted archives require the correct password. The CLI uses `--password-stdin`; an empty password is rejected.
- Volumes, recovery records, or uncommon encodings may be outside the current scope. RAR reading is Beta and RAR creation is unsupported.

For a compatibility report, provide the creating tool and version, format options, ZiFile version, and a public minimal reproducer when possible. Never upload customer data, passwords, or private archives.

## Extraction is rejected by safety policy

Archives containing `..`, absolute paths, device names, case collisions, links, or abnormal expansion are rejected. This often signals corruption, incompatibility, or hostile content. ZiFile has no switch to disable these safety boundaries. A trusted tool may be used to inspect structure in a controlled environment, but never overwrite an important destination.

ZiFile also rejects an extraction root or existing parent directory that is a symbolic link, junction, or reparse point. Choose a normal directory so output cannot be redirected outside the selected destination.

## The Create button is unavailable

- A gzip, zstd, xz, lzma, bzip2, lz4, or Brotli stream requires exactly one existing file and cannot accept a directory.
- ZIP, 7z, and TAR compositions accept multiple files and folders.
- Creation is blocked before the save dialog when sources are empty, no longer exist, are symbolic links, or the output is invalid.
- RAR is read-only and cannot be selected as a creation format.

## An operation is slow or cancelled

Large archives are protected by entry-count, expanded-size, compression-ratio, and memory limits; lists are paged in groups of 500. Cancellation waits for a Worker cancellation point and terminates the Worker after a timeout. Save the error text and reopen the application after a failure rather than repeatedly retrying the same suspicious archive.

## The Worker exits unexpectedly

ZiFile converts Worker crashes, protocol failures, and resource-limit exits into operation errors while keeping the desktop process available. Reopening the archive starts a new Worker. For a stable reproducer, record the app version, operation, format, file size, and exact error, then follow the safe reporting rules for a minimal sample.

## A development package will not install

An unsigned `.Dev` MSIX is not a production artifact, and some Windows versions reject its Publisher. Do not import a test root, disable SmartScreen, or weaken system trust policy. Use the complete runnable directory for development verification and wait for a publicly trusted or Microsoft Store package for production installation.

The Explorer menu can appear only after a formal package installs and activates the Shell extension. The current Release Candidate does not treat registration markup as proof of trusted installation or lifecycle behavior.

## Report safely

Use [GitHub Issues](https://github.com/ax2/zifile/issues) for ordinary defects. Follow the [security policy](https://github.com/ax2/zifile/security/policy) for suspected vulnerabilities, and never publish unpatched details, malicious archives, credentials, or private data in a public issue. Redact local usernames and paths from logs when needed.
