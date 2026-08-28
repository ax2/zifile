---
title: Code signing policy
description: ZiFile's SignPath Foundation application, signing roles, build provenance, and distribution policy.
---

## Current status

ZiFile is preparing an application for SignPath Foundation's free open-source code-signing service. Until the application is accepted, the repository does not describe any build as signed by SignPath Foundation and does not connect an unapproved provider to the production release gate.

The required service attribution is:

> Free code signing provided by SignPath.io, certificate by SignPath Foundation.

The canonical public policy is [`CODE-SIGNING-POLICY.md`](https://github.com/ax2/zifile/blob/main/CODE-SIGNING-POLICY.md).

## Roles and approval

- Committers and reviewers: [@ax2](https://github.com/ax2), the current repository owner and CODEOWNERS entry.
- Release approver: [@ax2](https://github.com/ax2).

Source code, build scripts, packaging, and signing configuration changes are reviewed through GitHub pull requests. The release approver checks the source revision, version, architecture, artifact scope, provenance, and post-signing verification before approving a signing request. Additional maintainers must be added to this page and the canonical policy before receiving signing access.

GitHub and signing-service accounts use multi-factor authentication. Signing credentials, private keys, and tokens are never committed or retained in releases, issues, pull requests, documentation archives, or ordinary build artifacts.

## Build and distribution boundary

Only ZiFile binaries built from reviewed source and build configuration in this repository may be submitted for signing. GitHub Actions is the authoritative release build path; it retains the source revision, architecture, version, provenance, package audit, and SHA-256 evidence, and verifies artifacts again after signing.

The MSIX `Identity Publisher` must match the signing certificate subject. Store-bound packages retain their separate Partner Center identity and must not be replaced by another certificate identity without an explicit review. WinGet manifests use the final signed artifact when calculating and recording the installer hash.

## Privacy

ZiFile does not transfer user files, archive contents, paths, or passwords to the signing service or to ZiCode. See the [public privacy statement](https://ax2.github.io/zifile/en/product/privacy/).

> This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.

## Application

[Apply for a free SignPath Foundation subscription](https://signpath.org/apply.html)
