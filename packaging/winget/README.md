# WinGet packaging

## Code signing policy

Free code signing provided by SignPath.io, certificate by SignPath Foundation.
The application is pending and this statement is not a claim that any current
artifact has been signed by SignPath Foundation. WinGet manifests must use the
final verified artifact and its post-signing SHA-256. See the [canonical Code
signing policy](../../CODE-SIGNING-POLICY.md).

ZiFile uses a schema 1.12 multi-file manifest for the signed x64 and ARM64
MSIX packages published by a GitHub release. Generate a submission candidate
only after the release URLs and SHA-256 values are final:

```powershell
./packaging/winget/Generate-Manifests.ps1 `
  -Version 1.0.0 `
  -X64InstallerUrl https://github.com/ax2/zifile/releases/download/v1.0.0/ZiFile-1.0.0.0-windows-x64.msix `
  -X64InstallerSha256 <64-hex-digits> `
  -Arm64InstallerUrl https://github.com/ax2/zifile/releases/download/v1.0.0/ZiFile-1.0.0.0-windows-arm64.msix `
  -Arm64InstallerSha256 <64-hex-digits>
```

The generated tree is written under the community-repository-compatible path
`target/winget/manifests/z/ZiCode/ZiFile/<version>/`. Before publication, verify
the generated URLs and hashes against the signed local MSIX files:

```powershell
./packaging/winget/Test-Manifests.ps1 `
  -ManifestDirectory target/winget/manifests/z/ZiCode/ZiFile/1.0.0 `
  -Version 1.0.0 `
  -X64InstallerPath <signed-x64.msix> `
  -Arm64InstallerPath <signed-arm64.msix>
```

The resulting directory is ready for `winget validate --manifest <directory>`.
The installer manifest declares the same 24 open extensions as
`OPEN_ARCHIVE_EXTENSIONS`, including RAR/CAB and comic/TAR aliases; the
preflight rejects any metadata drift before official validation.
Submission to `microsoft/winget-pkgs`
remains a deliberate release action after package signing and installation
testing; the generator never opens a pull request by itself.
