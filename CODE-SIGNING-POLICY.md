# Code signing policy

ZiFile is an open-source Windows archive and file utility maintained by ZiCode
and hosted at <https://github.com/ax2/zifile>.

## SignPath Foundation application

This project is applying for the following open-source service:

> Free code signing provided by SignPath.io, certificate by SignPath Foundation.

The application is not an approval or a claim that any current release is
signed by SignPath Foundation. Until an application is accepted and a signing
request is independently verified, releases must be described as unsigned,
development-signed, or signed by the actual provider shown in their evidence.

SignPath Foundation's certificate is issued in the Foundation's name. Any
MSIX artifact signed through that service must use an `Identity Publisher`
value that matches the certificate subject. The Store-bound package has a
separate Partner Center identity and must not be silently substituted with a
different publisher identity. The release workflow will keep these channels
separate until the identity and distribution plan are explicitly reviewed.

## Roles and review

- Committers and reviewers: [@ax2](https://github.com/ax2), the current
  repository owner and CODEOWNERS entry.
- Release approvers: [@ax2](https://github.com/ax2).

Changes to source code, build scripts, packaging, and signing configuration
are reviewed through GitHub pull requests. A release approver must inspect the
source revision, build result, artifact scope, version, and post-signing
verification before approving a signing request. Additional maintainers must
be added to this list before receiving signing access.

Project maintainers use multi-factor authentication for GitHub and any
signing-service account. Signing credentials and private keys are never stored
in the repository, release assets, issues, pull requests, documentation
archives, or ordinary build artifacts.

## Build and artifact boundary

Only ZiFile binaries built from this repository's reviewed source and build
configuration may be submitted for signing. GitHub Actions is the authoritative
build path for release artifacts. The workflow retains source revision,
architecture, version, provenance, package audit, and SHA-256 evidence, and
verifies the signature after signing.

The project does not use a SignPath Foundation certificate to sign unrelated
third-party projects or upstream binaries. Third-party dependencies remain
identified in the dependency and SBOM records.

The exact distribution artifact is signed before its SHA-256 is calculated.
WinGet manifests therefore refer to the final signed artifact, not to a
pre-signing build output.

## Distribution channels

- GitHub Releases: only artifacts whose signature and provenance are verified
  by the release workflow may be presented as trusted release artifacts.
- WinGet: the submitted installer and its manifest hash must correspond to the
  same final artifact. MSIX submissions require a valid signature; EXE/MSI
  submissions must also pass unattended-installation and repository validation.
- Microsoft Store: Store-bound MSIX packages retain the Partner Center package
  identity. Microsoft Store may apply the final Store-channel signature after
  certification.

## Privacy

ZiFile does not transfer user files, archive contents, paths, or passwords to
the signing service or to ZiCode. The public application privacy statement is
available in [English](https://ax2.github.io/zifile/en/product/privacy/) and
[Simplified Chinese](https://ax2.github.io/zifile/product/privacy/).

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

## Policy changes

Changes to signing providers, certificate identity, release roles, artifact
scope, or approval rules require a reviewed pull request and an update to this
policy before the changed signing path is used.
