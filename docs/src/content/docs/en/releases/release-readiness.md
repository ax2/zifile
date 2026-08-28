---
title: 1.0 release readiness
description: Machine-readable stable-tag gates and current evidence boundaries.
---

[`release/readiness.json`](https://github.com/ax2/zifile/blob/main/release/readiness.json) is the single structured status for the 1.0 external gates. Its current status is `candidate`: all 11 gates remain `pending`, and plans, existing scripts, or unsigned exercises are not presented as passing evidence.

## Stable-tag rule

Normal CI, manual Release rehearsals, and hyphenated prerelease tags validate the manifest structure. A stable tag also runs `Test-ReleaseReadiness.ps1 -RequireReleaseReady` and fails before build or publication while any gate remains `pending`. Changing a gate to `passed` requires an evidence URL from this repository's Actions, issues, pull requests, or releases.

## Current 11 gates

1. Freeze public contracts on the 1.0 commit.
2. Complete an undisturbed real-window multi-operation queue run.
3. Prove trusted MSIX install, launch, association, upgrade, Repair, and uninstall.
4. Run the candidate on physical ARM64 Windows.
5. Complete Narrator, keyboard, Chinese IME, high-contrast, and DPI gates and select the default UI.
6. Capture bilingual Store screenshots from the formally signed candidate.
7. Produce formal x64 and ARM64 WACK reports.
8. Complete Microsoft Store submission and certification.
9. Obtain acceptance in the WinGet community repository.
10. Reserve the Partner Center name and obtain the formal Package Identity.
11. ADR-0006 has removed PFX and wired cloud-HSM signing/post-signing audit; real-certificate dual-architecture signing, trusted lifecycle, and revocation evidence remain.

This page explains the policy; the JSON manifest and machine gate decide whether a stable tag is allowed. `candidate` does not mean Store-ready, signed, or releasable.
