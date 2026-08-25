---
title: Microsoft Store and WinGet checklist
description: External distribution gates, required material, and repeatable checks.
---

## Automated foundation

- x64/ARM64 runnable directories, standalone EXEs, and MSIX builds.
- Optional PFX signing with secrets injected only during the job.
- SHA-256 files, CycloneDX JSON SBOMs, and GitHub build provenance.
- WinGet 1.12 multi-file manifests with both architectures, associations, and bilingual metadata.
- Package audits for identity, publisher, version, minimum OS, four PE architectures, associations, CLI alias, sensitive-file/ZIP absence, and signature state.
- Tag policy rejects missing official credentials, `.Dev` identities, and unsigned OID publishers.
- Structured English and Simplified Chinese listing copy, privacy statements, and certification notes are present; CI validates Partner Center limits for descriptions, features, keywords, licensing, and HTTPS URLs.
- The bilingual Desktop screenshot manifest has explicit `draft/complete` state. Its gate validates PNG/IHDR, the 1366×768 minimum (or portrait equivalent), the 50 MB limit, SHA-256, path containment, order, scenarios, and 200-character captions. Tagged publishing requires at least four screenshots per language and fails before packaging otherwise.
- The Store smoke test dynamically creates four real PNGs for each locale, proves that a complete manifest passes, and proves that undersized and duplicate images are rejected. The repository manifest remains an explicit zero-image `draft`; test images are never presented as formal assets.

A candidate manifest using real Release hashes passed local `winget validate`; this does not mean it has been submitted or accepted.

## Development-package boundary

Unsigned `.Dev` packages use Microsoft's fixed OID and require Windows 11 build 26100. The current test machine still rejects that package with `0x80080204`. A temporary self-signed exercise proved manifest/subject/SignTool consistency but installation stopped at untrusted root `0x800B0109`. No root, key, certificate, or package registration was retained. Neither exercise replaces trusted signing or Partner Center identity.

The trusted lifecycle workflow also builds a self-contained test helper pinned to Windows App SDK 1.8. When Repair is supported, it calls the same `RepairPackageAsync` operation as Windows Settings after upgrade and requires a package LocalState sentinel to survive; Reset must then delete that sentinel. A non-mutating probe on the current Windows 25H2 build 26200 reports `repair_supported=false`, so the gate records unavailable capability and never presents Reset as Repair.

## External gates before first submission

1. Register a Partner Center developer account and reserve `ZiFile`.
2. Store the assigned Package Identity Name and Publisher in GitHub Secrets.
3. Obtain trusted signing for GitHub/WinGet; Store distribution is signed by Microsoft.
4. Rebuild both architectures with official identity and test install, launch, association, upgrade, repair, and uninstall.
5. Run WACK in an administrator's interactive session and complete keyboard, Narrator, high-contrast, DPI, and Chinese IME checks.
6. Review the prepared bilingual listing copy, privacy statements, and certification notes; deploy the public privacy pages, capture localized desktop screenshots from the signed candidate, and complete age rating and markets.
7. Submit validated MSIX packages; after a public Release, generate and submit the WinGet PR.

Until these external gates pass, no Alpha artifact may be called Store-ready or signed.
