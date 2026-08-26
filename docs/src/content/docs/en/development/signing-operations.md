---
title: Production signing operations
description: Provisioning, approval, rehearsal, rotation, revocation, and evidence for DigiCert Binary Signing.
---

## Current boundary

This runbook implements the DigiCert Binary Signing baseline in [ADR-0006](/zifile/en/architecture/adr-0006-release-signing/). The repository has a protected `production-signing` Environment, cloud-signing Action, and post-signing verifier, but no organization account, publicly trusted certificate, or credentials are configured. `production-cloud-hsm-signing` remains `pending` until a real run produces evidence.

The code-signing private key must remain in the cloud HSM. `SM_CLIENT_CERT_FILE_B64` is a client-authentication certificate for service access, not the code-signing private key; it is still a secret and must never appear in the repository, logs, issues, pull requests, archives, or long-lived local files.

## Configuration inventory

| Location | Name | Classification | Purpose |
| --- | --- | --- | --- |
| Repository Variable | `ZIFILE_MSIX_IDENTITY` | Non-secret | Exact Package Identity Name assigned by Partner Center |
| Repository Variable | `ZIFILE_MSIX_PUBLISHER` | Non-secret | Exact certificate Subject and MSIX Publisher |
| Environment Variable | `SM_HOST` | Non-secret | DigiCert service endpoint |
| Environment Variable | `SM_KEYPAIR_ALIAS` | Non-secret | Keypair approved for ZiFile production releases |
| Environment Secret | `SM_API_KEY` | Secret | Least-privilege automation API key |
| Environment Secret | `SM_CLIENT_CERT_FILE_B64` | Secret | Base64 client-authentication PKCS#12 |
| Environment Secret | `SM_CLIENT_CERT_PASSWORD` | Secret | Client-authentication certificate password |

Initial provisioning requires organization validation, confirmed certificate purpose and quota, a least-privilege service user, and enabled audit logs. Keep a required reviewer on `production-signing`; deployment policies permit only `v*` tags and an explicitly named rehearsal branch. Never print a secret, open deployment to every branch, or fall back to an exportable PFX to diagnose a failure.

## First real rehearsal

1. Manually run Release with `signing_provider=digicert-stm`. Verify the source commit and workspace version, then approve the Environment deployment.
2. Require both x64 and ARM64 `Sign Windows` jobs to succeed. Missing input, nonzero Action exit, invalid signature, missing timestamp, or Publisher mismatch must stop the run.
3. Download `signed-windows-x64` and `signed-windows-arm64`. Verify five entries in `.signing.json`, `Valid` in `.audit.json`, `SHA256SUMS-*`, and GitHub provenance. Do not retain artifact ZIPs; archive only extracted evidence or complete runnable directories.
4. Run trusted install, launch, Explorer, upgrade, Repair/Reset, and uninstall gates on a clean x64 machine and repeat on physical Windows ARM64 hardware. Then run WACK readiness and formal WACK.
5. Update `release/readiness.json` only after dual-architecture signing, lifecycle, revocation rehearsal, and evidence URLs pass. CI wiring alone cannot clear the gate.

## Production release

Freeze the version, CHANGELOG, Store screenshots, and all 11 readiness evidence items before creating the exact `v<workspace-version>` tag. The reviewer verifies commit, version, certificate subject, Keypair Alias, and change scope before approval. After publishing, download the public GitHub Release files again and verify SHA-256, signatures, timestamps, provenance, WinGet hashes, and the absence of ZIP or authentication material.

## Rotation

- Rotate the API key and client-authentication certificate separately. Create a least-privilege replacement, update the Environment secret, run and verify one `digicert-stm` rehearsal, then revoke the old credential. Record only date, owner, run URL, and outcome.
- Before rotating the code-signing certificate or Keypair, confirm that the new Subject exactly matches `ZIFILE_MSIX_PUBLISHER`. Complete dual-architecture signing and upgrade tests before production cutover. Retain evidence for older timestamped signatures.
- Periodically review service users, Environment reviewers, deployment branches, signing quota, and audit logs. Departure, permission change, or provider alert triggers an immediate review.

## Emergency stop and revocation

If an API key, client-authentication credential, service account, Keypair, or release might be abused, perform an emergency stop immediately: cancel active Release runs, disable `production-signing` or remove its allowed policies, revoke the API key/client certificate, and disable the Keypair in DigiCert. Do not wait for root-cause analysis before stopping signing.

Preserve non-secret audit logs and affected hashes, then correlate GitHub Actions, DigiCert signing logs, and Release downloads. Remove or clearly mark an affected unverified Release and assess certificate revocation and user notification. New credentials and Keypair must pass the full rehearsal before re-enabling the Environment. Exercise revocation through the provider's test process or a dedicated test certificate; never revoke the production certificate merely to rehearse.

## Minimum evidence set

Retain source commit/tree, workflow and job URLs, architecture, version, Identity, Publisher, signer/timestamp thumbprints, SHA-256 for all five files, signing JSON, package audit, provenance, lifecycle/WACK results, and reviewer. Never retain API keys, passwords, client-certificate contents, cookies, tokens, or a code-signing private key.
