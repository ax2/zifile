# WinGet packaging

## Code signing policy

Free code signing provided by SignPath.io, certificate by SignPath Foundation.
The application is pending and this statement is not a claim that any current
artifact has been signed by SignPath Foundation. WinGet manifests must use the
final verified artifact and its post-signing SHA-256. See the [canonical Code
signing policy](../../CODE-SIGNING-POLICY.md).

ZiFile uses a schema 1.12 multi-file manifest for the signed all-in-one MSIX
bundle published by a GitHub release. The manifest keeps x64 and ARM64 entries
for WinGet selection, both pointing to the same bundle asset. Generate a
submission candidate only after the release URL and SHA-256 value are final:

```powershell
./packaging/winget/Generate-Manifests.ps1 `
  -Version 1.0.0 `
  -BundleInstallerUrl https://github.com/ax2/zifile/releases/download/v1.0.0/ZiFile-1.0.0.0-windows.msixbundle `
  -BundleInstallerSha256 <64-hex-digits>
```

The generated tree is written under the community-repository-compatible path
`target/winget/manifests/z/ZiCode/ZiFile/<version>/`. Before publication, verify
the generated URLs and hashes against the signed local MSIX files:

```powershell
./packaging/winget/Test-Manifests.ps1 `
  -ManifestDirectory target/winget/manifests/z/ZiCode/ZiFile/1.0.0 `
  -Version 1.0.0 `
  -BundleInstallerPath <signed-all-in-one.msixbundle>
```

The resulting directory is ready for `winget validate --manifest <directory>`.
The installer manifest declares the same 31 open extensions as
`OPEN_ARCHIVE_EXTENSIONS`, including RAR/CAB and comic/TAR aliases. When a
local bundle is supplied, the preflight also opens both nested MSIX packages
and requires their `Identity.Name` and four-part `Identity.Version` to match
the WinGet candidate. Development identities such as `ZiCode.ZiFile.Dev` are
rejected before official validation, because a development package is not the
final package submitted to the community repository.

The ordinary unsigned GitHub Release workflow may pass
`-AllowDevelopmentIdentity` while producing internal evidence for the `.Dev`
package. That exception is explicit and workflow-only; do not use it when
preparing a WinGet community submission.
Submission to `microsoft/winget-pkgs`
remains a deliberate release action after package signing and installation
testing; the generator never opens a pull request by itself.
