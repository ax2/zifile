---
title: "ADR-0006: Windows release signing"
description: Trusted-signing routes and credential boundaries for Store, GitHub, and WinGet.
---

## Status

Accepted on 2026-08-25; implementation updated on 2026-08-26. No external signing account has been provisioned, so this ADR decides the architecture, provider priority, and CI contract without claiming that a certificate or trusted signature exists.

## Decision

Microsoft Store submissions use the exact Partner Center Identity and Publisher and receive their final signature from the Store. GitHub Release and WinGet artifacts use an organization-validated, publicly trusted cloud-HSM signing service. The private key must never be exported to the repository, a developer machine, or a GitHub secret; GitHub provenance attestations complement but do not replace Windows Authenticode.

Artifact Signing Public Trust is preferred when ZiCode's contracting entity satisfies [Microsoft's regional and identity eligibility](https://learn.microsoft.com/azure/artifact-signing/quickstart). It must use a least-privilege CI identity and timestamping. The current official Action is `Azure/artifact-signing-action`; the former Trusted Signing name must not be used.

Until eligibility is proven, the implementation baseline is a publicly trusted organization-validated certificate through DigiCert Binary Signing and the official `digicert/code-signing-software-trust-action`. New integration uses the recommended simple-signing mode instead of the retiring legacy KSP/SignTool GitHub Action. Procurement must still verify entity validation, regional sales, pricing, quota, and simple-signing entitlement.

PFX handling has been removed from the Release workflow. The credential-free CI contract, post-signing audit, and post-signing provenance stages are implemented. The release gate remains closed until a real account signs artifacts and trusted install/upgrade and revocation exercises pass on the required architectures.

## Cost, renewal, and operations

| Route | Current public cost baseline | CI and custody | Renewal and constraint |
| --- | --- | --- | --- |
| Artifact Signing Basic | USD 9.99/month including 5,000 signatures; USD 0.005 per excess signature | Official GitHub Action, service HSM, and three-day short-lived certificates that require timestamping | Managed certificate lifecycle; Public Trust enrollment is region-limited |
| DigiCert Binary Signing | Formal quote required | Simple-signing Action; key remains in cloud HSM and CI uses API plus client authentication | Subscription and organization validation renew; quota, regional sale, and automation rights require written confirmation |

The Artifact Signing price is a public-page snapshot from 2026-08-25, not a purchasing quote; this document does not freeze an uncontracted DigiCert price. One publisher identity covers x64 and ARM64; each architecture's artifacts still require independent signature verification.

## Acceptance criteria

The integration signs x64 and ARM64 standalone EXEs, the shell DLL, and MSIX with SHA-256 and an RFC 3161 timestamp. It then validates the operating-system trust chain and regenerates package audits, checksums, and provenance. Credentials never appear in logs; production signing is restricted to protected tags/environments with least privilege, approval, rotation, revocation, and emergency-stop procedures.

[Microsoft's current list](https://learn.microsoft.com/azure/artifact-signing/quickstart) makes Public Trust available to organizations in the United States, Canada, the European Union, the United Kingdom, Australia, New Zealand, Japan, South Korea, Singapore, Switzerland, Norway, and Israel; individual eligibility remains narrower. Resource region and entity eligibility are separate constraints. DigiCert is an implementation baseline rather than purchasing authorization; an equivalent publicly trusted CA cloud-signing service may replace it if commercial validation fails.

## CI credential boundary

The protected `production-signing` GitHub Environment owns approval and production scope. Build-time `ZIFILE_MSIX_IDENTITY` and `ZIFILE_MSIX_PUBLISHER` are non-secret repository variables. Sign-only `SM_HOST` and `SM_KEYPAIR_ALIAS` are non-secret Environment variables. `SM_API_KEY`, the Base64 client-authentication certificate, and its password are Environment secrets. The client certificate authenticates to the service; it is not an export of the code-signing private key, and it is deleted from the runner temporary directory after use. Tags always use cloud signing. Manual Release runs select either `none` for unsigned validation or `digicert-stm` for a protected real-signing rehearsal.
