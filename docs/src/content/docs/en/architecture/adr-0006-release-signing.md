---
title: "ADR-0006: Windows release signing"
description: Trusted-signing routes and credential boundaries for Store, GitHub, and WinGet.
---

## Status

Accepted on 2026-08-25. No external signing account has been provisioned, so this ADR decides the architecture and provider priority without claiming a certificate or trusted signature exists.

## Decision

Microsoft Store submissions use the exact Partner Center Identity and Publisher and receive their final signature from the Store. GitHub Release and WinGet artifacts use an organization-validated, publicly trusted cloud-HSM signing service. The private key must never be exported to the repository, a developer machine, or a GitHub secret; GitHub provenance attestations complement but do not replace Windows Authenticode.

Artifact Signing Public Trust is preferred when ZiCode's contracting entity satisfies Microsoft's regional and identity eligibility. It must use a least-privilege CI identity and timestamping. Until that eligibility is proven, the implementation baseline is a DigiCert Software Trust Manager public OV certificate through its KSP/SignTool and GitHub Actions integration. Procurement must still verify entity validation, regional sales, pricing, and signing quotas.

The current PFX workflow is scaffolding and is not approved for a 1.0 tag. The release gate remains closed until cloud signing, post-signing audit, trusted install/upgrade, and revocation exercises pass on the required architectures.

## Cost, renewal, and operations

| Route | Current public cost baseline | CI and custody | Renewal and constraint |
| --- | --- | --- | --- |
| Artifact Signing Basic | USD 9.99/month including 5,000 signatures; USD 0.005 per excess signature | Official GitHub Action, service HSM, and three-day short-lived certificates that require timestamping | Managed certificate lifecycle; Public Trust enrollment is region-limited |
| DigiCert cloud signing | Public code-signing purchase page starts around USD 44/month; an STM/OV quote is still required | STM/KSP/SignTool; key remains in cloud HSM and CI uses API plus client authentication | Subscription and organization validation renew; quota, regional sale, and automation rights require written confirmation |

Prices are a public-page snapshot from 2026-08-25, not a purchasing quote. One publisher identity covers x64 and ARM64; each architecture's artifacts still require independent signature verification.

## Acceptance criteria

The integration signs x64 and ARM64 standalone EXEs, the shell DLL, and MSIX with SHA-256 and an RFC 3161 timestamp. It then validates the operating-system trust chain and regenerates package audits, checksums, and provenance. Credentials never appear in logs; production signing is restricted to protected tags/environments with least privilege, approval, rotation, revocation, and emergency-stop procedures.

Microsoft currently limits Artifact Signing Public Trust enrollment to organizations in the United States, Canada, the European Union, and the United Kingdom, with narrower individual eligibility. DigiCert is therefore an implementation baseline rather than purchasing authorization; an equivalent publicly trusted CA cloud-signing service may replace it if commercial validation fails.
