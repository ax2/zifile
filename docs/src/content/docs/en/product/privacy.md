---
title: Privacy statement
description: How ZiFile handles files, passwords, settings, and diagnostics locally.
---

Effective and last updated: August 25, 2026.

ZiFile is published by ZiCode and is an open-source Windows archive utility licensed under the MIT License. This statement applies to the ZiFile desktop app, command-line tool, isolated Worker, and File Explorer extension.

## Data we do not collect

ZiFile requires no account and contains no advertising, analytics SDK, default telemetry, cloud synchronization, or automatic crash upload. The app does not send file names, paths, file contents, archive contents, passwords, or usage activity to ZiCode or third-party services.

## Local processing

Files and archives that the user selects or drops are processed only on the device. Input files, created archives, and extracted results remain in locations selected by the user on the local file system. ZiCode does not receive this content and sets no remote retention period for it.

ZIP and 7z passwords are passed and used only for the current operation. They are not written to settings, logs, or command-line arguments. The CLI reads passwords from standard input. The app does not intentionally retain a password after the operation; as with any desktop process, data can exist briefly in memory while the process is running.

ZiFile stores only non-sensitive preferences such as interface language and theme on the device. Users can clear these settings through app controls or Windows app reset/uninstall functions.

## Network and third parties

The current MSIX declares only the full-trust desktop capability and does not declare the Internet client capability. ZiFile provides no in-app network service. Microsoft Store, GitHub, or WinGet may process network and account data under their own privacy terms when users download or update the app; that activity is separate from data collection by ZiFile itself.

ZiFile does not sell, rent, or share personal data because the app does not collect that data for ZiCode. Open-source dependencies and their licenses are recorded in the project repository and release materials.

## Children, changes, and contact

ZiFile is a general-purpose file utility, is not directed to children, and does not knowingly collect children's data.

If a future optional feature requires networking or data collection, this statement will be updated before that feature is released and will describe its purpose, scope, retention, and controls. The revision date will remain visible at the top of this page.

For privacy questions or deletion requests, contact ZiCode through [ZiFile GitHub Issues](https://github.com/ax2/zifile/issues). Because the current app does not upload user files or personal data to ZiCode, ZiCode normally holds no app data that it can delete on a user's behalf.
