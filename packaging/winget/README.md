# WinGet packaging

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

The generated tree is written under `target/winget/` and is ready for
`winget validate --manifest <directory>`. Submission to `microsoft/winget-pkgs`
remains a deliberate release action after package signing and installation
testing; the generator never opens a pull request by itself.
