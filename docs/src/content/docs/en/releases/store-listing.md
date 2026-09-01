---
title: Microsoft Store listing copy
description: Partner Center-ready English metadata and asset status for ZiFile.
---

This page is the readable counterpart of `packaging/store/listing.en-US.json`. The JSON source is authoritative for automated field-limit validation. Export the latest Partner Center template and compare it again before submission.

## Positioning and short description

Product name: ZiFile

Category: Utilities & tools

Pricing: Free; no ads, in-app purchases, or subscriptions

Short description: A modern open-source archive utility for Windows 10 and 11. Browse and safely extract RAR, CAB, ZIP, 7z, and TAR; create open formats; use encryption and the command line.

## Full description

ZiFile is a modern open-source archive utility for Windows with clear workflows and cautious security defaults.

Browse archive contents, extract only selected entries, or create ZIP, 7z, TAR, fixed-MSZIP Windows CAB archives, and common compressed streams from files and folders. RAR 1.3 through RAR 7 can be browsed, tested, and extracted but not created. CAB does not support passwords, update, or rename operations and cannot preserve empty directories. ZIP and 7z support AES encryption, and passwords are never written to logs or settings.

ZiFile blocks path traversal, link escapes, reserved Windows names, unsafe overwrites, and excessive archive expansion. Long-running work executes in an isolated background process with visible progress and cancellation.

The app provides English and Simplified Chinese interfaces, drag-and-drop, file associations, and a command-line tool. All archive processing happens locally, with no account, cloud service, advertising, or telemetry.

ZiFile is open source under the MIT License and supports x64 and ARM64 Windows devices.

## Features and keywords

Partner Center adds feature bullets automatically. Paste each line without a bullet marker.

1. Browse and extract RAR, CAB, ZIP, 7z, TAR, and common compressed streams
2. AES encryption for ZIP and 7z without saving passwords to logs or settings
3. Protection from path traversal, link escapes, unsafe overwrites, and archive bombs
4. Isolated background operations with visible progress and cancellation
5. Drag-and-drop, file associations, and a command-line tool
6. English and Simplified Chinese interfaces on x64 and ARM64

Keywords: archive, compression, extract, ZIP, 7z, CAB, file utility.

Leave “What's new in this version” blank for the first submission. Use `MIT License` for applicable license terms and `ZiCode` for Developed by.

## URLs and certification notes

- Support: `https://github.com/ax2/zifile/issues`
- Website: `https://ax2.github.io/zifile/`
- Privacy: `https://ax2.github.io/zifile/en/product/privacy/`

Certification notes should explain that the app processes only user-selected local files; an isolated Worker included in the package performs long operations; the Explorer extension only starts the visible desktop's create or extract-to-matching-folder flow and never parses archives, performs compression, or handles passwords itself; passwords are passed transiently through in-memory IPC or standard input; RAR is read-only and RAR creation is unsupported; and the MSIX does not declare the Internet client capability.

## Asset status

MSIX package logos are generated automatically and covered by package audit. The recommended 300×300 1:1 Partner Center app tile listing icon is also staged under `packaging/store/listing-assets/`; CI verifies its dimensions, format, pinned hash, and generator consistency. The Store requires at least one PC screenshot and Microsoft recommends at least four. Before submission, capture localized “home/open archive,” “create archive,” “extraction options,” and “background progress or completion” views from a signed candidate, recording viewport, scale, theme, and version. Completed listing copy and icon preparation do not mean the Store submission is complete.
