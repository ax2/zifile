---
title: Microsoft Store and WinGet checklist
description: External distribution gates, required material, and repeatable checks.
---

## Automated foundation

- x64/ARM64 runnable directories, standalone EXEs, and MSIX builds.
- The MSIX contains Microsoft's complete minimum high-DPI icon matrix: 58 hash-pinned PNGs covering Store scale variants, 100/200/400 percent 44px and 150px resources, and 14 target sizes in default, dark-unplated, and light-unplated forms for taskbar, Start, and Search. The post-build package audit rechecks every hash from the unpacked candidate.
- The desktop EXE and Explorer shell entry share one reviewed multi-resolution Win32 ICO with 16, 24, 32, 48, and 256 pixel 32-bit PNG frames. Machine gates verify generation, directory structure, dimensions, and the unpacked package hash.
- Release has removed PFX and includes protected DigiCert Binary Signing simple signing, operating-system verification, timestamp enforcement, and post-signing audit; a real-account rehearsal still awaits external credentials.
- `Test-PartnerCenterIdentity.ps1` rejects a missing or partial identity tuple, malformed Name, `.Dev`, unsigned OID, invalid X.500 Publisher, and invalid Publisher Display Name before a tag or real-signing rehearsal compiles. All three formal values must come from Partner Center.
- SHA-256 files, CycloneDX JSON SBOMs, and GitHub build provenance.
- WinGet 1.12 multi-file manifests with both architectures, associations, bilingual metadata, and the community repository path `manifests/z/ZiCode/ZiFile/<version>/`.
- Tagged publishing runs `Test-Manifests.ps1` before upload. It requires all four schema types, official versioned GitHub Release HTTPS URLs, one x64 and one arm64 installer, and exact SHA-256 matches against the signed local MSIX files. Tampering or a wrong version path fails publishing.
- Windows CI also generates a deterministic candidate and runs the system `winget validate`, catching drift between ZiFile preflight and the official schema. The fixture does not download public assets, so it is not evidence of URL availability or repository acceptance.
- Package audits for identity, publisher, publisher display name, version, minimum OS, four PE architectures, associations, CLI alias, sensitive-file/ZIP absence, and signature state.
- Tags always enter the `production-signing` Environment, and publishing consumes only post-gate `signed-windows-*` artifacts. A `.Dev` identity, unsigned OID publisher, missing cloud input, invalid signature, or missing timestamp fails the run.
- Structured English and Simplified Chinese listing copy, privacy statements, and certification notes are present; CI validates Partner Center limits for descriptions, features, keywords, licensing, and HTTPS URLs, and rejects drift between JSON description paragraphs/features and the readable pages.
- Privacy URLs are fixed to the localized GitHub Pages policy routes. Normal CI checks both generated `index.html` files and privacy markers after the Astro build; after deployment, the Pages workflow requests the public HTTPS pages and requires HTTP 200, preventing a Store listing from referencing a 404 or unrelated page.
- The bilingual Desktop screenshot manifest has explicit `draft/complete` state. Its gate validates PNG/IHDR, the 1366×768 minimum (or portrait equivalent), the 50 MB limit, SHA-256, path containment, order, scenarios, and 200-character captions. Tagged publishing requires at least four screenshots per language and fails before packaging otherwise.
- The Store smoke test dynamically creates four real PNGs for each locale, proves that a complete manifest passes, and proves that undersized and duplicate images are rejected. The repository manifest remains an explicit zero-image `draft`; test images are never presented as formal assets.
- `Import-Screenshots.ps1` atomically imports a fixed four-scenario capture set for both locales, computes hashes and captions, and requires app version, Windows build, theme, scale, UTC time, source commit, and signed-candidate kind. It accepts only a draft destination and refuses to overwrite existing `assets`; invalid images or metadata cannot produce a complete manifest.

A candidate manifest using real Release hashes passed local `winget validate`; this does not mean it has been submitted or accepted.

## Development-package boundary

Unsigned `.Dev` packages use Microsoft's fixed OID and require Windows 11 build 26100. The current test machine still rejects that package with `0x80080204`. A temporary self-signed exercise proved manifest/subject/SignTool consistency but installation stopped at untrusted root `0x800B0109`. No root, key, certificate, or package registration was retained. Neither exercise replaces trusted signing or Partner Center identity.

The trusted lifecycle workflow downloads only `signed-windows-x64` from two signed Release runs, never the pre-signing `windows-x64` artifact. It also builds a self-contained test helper pinned to Windows App SDK 1.8. When Repair is supported, it calls the same `RepairPackageAsync` operation as Windows Settings after upgrade and requires a package LocalState sentinel to survive; Reset must then delete that sentinel. A non-mutating probe on the current Windows 25H2 build 26200 reports `repair_supported=false`, so the gate records unavailable capability and never presents Reset as Repair.

## External gates before first submission

These items are also tracked in machine-readable [`release/readiness.json`](https://github.com/ax2/zifile/blob/main/release/readiness.json) and [1.0 release readiness](/zifile/en/releases/release-readiness/). A stable tag requires evidence and `passed` status for every gate.

1. Register a Partner Center **Company** Windows developer account for the ZiCode legal entity and reserve `ZiFile`. Microsoft's May 2026 onboarding guidance says both Individual and Company accounts currently have no registration fee, and an Individual account cannot be converted directly to Company. Prefer D-U-N-S for company verification or provide the official business documents requested by the portal. A name reservation currently lasts three months, so record its expiry and reserve within the RC submission window.
2. Store the assigned Package Identity Name, Publisher, and developer-account Publisher Display Name verbatim as GitHub Repository Variables.
3. Complete DigiCert organization validation and certificate provisioning; configure host, keypair alias, API key, and client-authentication material in the protected Environment and run a dual-architecture manual signing rehearsal. Store distribution is signed by Microsoft.
4. Rebuild both architectures with official identity and test install, launch, association, upgrade, repair, and uninstall.
5. Run WACK in an administrator's interactive session; inspect taskbar, Start, Search, and association icons at 100%, 150%, 200%, and 400% scale; then complete keyboard, Narrator, high-contrast, and Chinese IME checks.
6. Review the prepared bilingual listing copy, privacy statements, and certification notes; deploy the public privacy pages, capture localized desktop screenshots from the signed candidate, and complete age rating and markets.
7. Submit validated MSIX packages; the public Release includes the locally post-signing-verified WinGet candidate, which must then pass official `winget validate` before the community-repository PR is submitted.

Until these external gates pass, no Alpha artifact may be called Store-ready or signed.

Before WACK, run `Test-WackReadiness.ps1 -ExpectedIdentityName $env:ZIFILE_MSIX_IDENTITY -ExpectedPublisher $env:ZIFILE_MSIX_PUBLISHER -ExpectedPublisherDisplayName $env:ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME -RequireReady` against the target MSIX and adjacent `.audit.json`. Passing readiness only proves the tool, session, architecture, exact Partner Center tuple, hash, minimum OS, and signature preconditions; the generated WACK report remains authoritative.

The WinGet community repository does not require Partner Center access, but a first contribution needs a GitHub account and may require completing the Microsoft CLA once when prompted by the CLA bot. Each PR contains one manifest set for one package version, and installer URLs must be stable, version-specific, and controlled by the publisher. See [Partner Center developer onboarding](https://learn.microsoft.com/windows/apps/publish/partner-center/open-a-developer-account) and the [WinGet first-time contributor checklist](https://github.com/microsoft/winget-pkgs/blob/master/doc/FirstContribution.md).
