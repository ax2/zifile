param(
    [Parameter(Mandatory)]
    [string]$Version,
    [Parameter(Mandatory)]
    [uri]$X64InstallerUrl,
    [Parameter(Mandatory)]
    [ValidatePattern('^[A-Fa-f0-9]{64}$')]
    [string]$X64InstallerSha256,
    [Parameter(Mandatory)]
    [uri]$Arm64InstallerUrl,
    [Parameter(Mandatory)]
    [ValidatePattern('^[A-Fa-f0-9]{64}$')]
    [string]$Arm64InstallerSha256,
    [uri]$BundleInstallerUrl,
    [ValidatePattern('^[A-Fa-f0-9]{64}$')]
    [string]$BundleInstallerSha256,
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\..\target\winget')
)

$ErrorActionPreference = 'Stop'
$identifier = 'ZiCode.ZiFile'
$fileExtensions = @(
    'zip', 'zipx', 'cbz', 'epub',
    '7z', 'cb7',
    'rar', 'cbr',
    'cab',
    'tar', 'cbt',
    'gz', 'tar.gz', 'tgz',
    'zst', 'tar.zst', 'tzst',
    'xz', 'tar.xz', 'txz', 'tar.lzma', 'lzma',
    'bz', 'bz2', 'tar.bz2', 'tbz', 'tbz2',
    'lz4', 'br'
)
$fileExtensionYaml = ($fileExtensions | ForEach-Object { "- $_" }) -join "`n"
$versionPattern = '^\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?$'
if ($Version -notmatch $versionPattern) {
    throw "Version '$Version' is not a supported ZiFile release version."
}
$useBundle = $null -ne $BundleInstallerUrl -or -not [string]::IsNullOrWhiteSpace($BundleInstallerSha256)
if ($useBundle -and ($null -eq $BundleInstallerUrl -or [string]::IsNullOrWhiteSpace($BundleInstallerSha256))) {
    throw 'BundleInstallerUrl and BundleInstallerSha256 must be supplied together.'
}
if ($useBundle) {
    $expectedPrefix = "https://github.com/ax2/zifile/releases/download/v$Version/"
    if ($BundleInstallerUrl.Scheme -ne 'https' -or
        -not $BundleInstallerUrl.AbsoluteUri.StartsWith($expectedPrefix, [StringComparison]::Ordinal) -or
        -not $BundleInstallerUrl.AbsolutePath.EndsWith('.msixbundle', [StringComparison]::OrdinalIgnoreCase)) {
        throw "Bundle installer URL must use the versioned ZiFile GitHub Release path '$expectedPrefix' and name an MSIX bundle."
    }
}
foreach ($installer in @(
    @{ Architecture = 'x64'; Url = $X64InstallerUrl },
    @{ Architecture = 'arm64'; Url = $Arm64InstallerUrl }
)) {
    $expectedPrefix = "https://github.com/ax2/zifile/releases/download/v$Version/"
    if ($installer.Url.Scheme -ne 'https' -or
        -not $installer.Url.AbsoluteUri.StartsWith($expectedPrefix, [StringComparison]::Ordinal)) {
        throw "$($installer.Architecture) installer URL must use the versioned ZiFile GitHub Release path '$expectedPrefix'."
    }
    if (-not $installer.Url.AbsolutePath.EndsWith("windows-$($installer.Architecture).msix", [StringComparison]::OrdinalIgnoreCase)) {
        throw "$($installer.Architecture) installer URL must name the matching windows-$($installer.Architecture).msix package."
    }
}

$output = [IO.Path]::GetFullPath([IO.Path]::Combine([string[]]@(
    $OutputRoot, 'manifests', 'z', 'ZiCode', 'ZiFile', $Version
)))
New-Item -ItemType Directory -Path $output -Force | Out-Null
$manifestX64Url = if ($useBundle) { $BundleInstallerUrl.AbsoluteUri } else { $X64InstallerUrl.AbsoluteUri }
$manifestArm64Url = if ($useBundle) { $BundleInstallerUrl.AbsoluteUri } else { $Arm64InstallerUrl.AbsoluteUri }
$manifestX64Sha256 = if ($useBundle) { $BundleInstallerSha256 } else { $X64InstallerSha256 }
$manifestArm64Sha256 = if ($useBundle) { $BundleInstallerSha256 } else { $Arm64InstallerSha256 }

@"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.version.1.12.0.schema.json
PackageIdentifier: $identifier
PackageVersion: $Version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.12.0
"@ | Set-Content -LiteralPath (Join-Path $output "$identifier.yaml") -Encoding utf8NoBOM

@"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.installer.1.12.0.schema.json
PackageIdentifier: $identifier
PackageVersion: $Version
Platform:
- Windows.Desktop
MinimumOSVersion: 10.0.19041.0
InstallerType: msix
Scope: user
UpgradeBehavior: install
FileExtensions:
$fileExtensionYaml
Installers:
- Architecture: x64
  InstallerUrl: $manifestX64Url
  InstallerSha256: $($manifestX64Sha256.ToUpperInvariant())
- Architecture: arm64
  InstallerUrl: $manifestArm64Url
  InstallerSha256: $($manifestArm64Sha256.ToUpperInvariant())
ManifestType: installer
ManifestVersion: 1.12.0
"@ | Set-Content -LiteralPath (Join-Path $output "$identifier.installer.yaml") -Encoding utf8NoBOM

@"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.defaultLocale.1.12.0.schema.json
PackageIdentifier: $identifier
PackageVersion: $Version
PackageLocale: en-US
Publisher: ZiCode
PublisherUrl: https://zicode.com/
PublisherSupportUrl: https://github.com/ax2/zifile/issues
Author: ZiCode
PackageName: ZiFile
PackageUrl: https://github.com/ax2/zifile
License: MIT
LicenseUrl: https://github.com/ax2/zifile/blob/main/LICENSE
ShortDescription: A modern, safe archive manager for Windows written primarily in Rust.
Description: Create, browse, verify, and safely extract common archive and compression formats from a modern Windows desktop interface or command line.
Moniker: zifile
Tags:
- archive
- compression
- rust
- sevenzip
- tar
- zip
ManifestType: defaultLocale
ManifestVersion: 1.12.0
"@ | Set-Content -LiteralPath (Join-Path $output "$identifier.locale.en-US.yaml") -Encoding utf8NoBOM

@"
# yaml-language-server: `$schema=https://aka.ms/winget-manifest.locale.1.12.0.schema.json
PackageIdentifier: $identifier
PackageVersion: $Version
PackageLocale: zh-CN
Publisher: ZiCode
PackageName: ZiFile
ShortDescription: 以 Rust 为主实现的现代、安全 Windows 压缩文件管理器。
Description: 通过现代 Windows 桌面界面或命令行创建、浏览、校验并安全解压常见归档与压缩格式。
Tags:
- 压缩
- 归档
- 解压
ManifestType: locale
ManifestVersion: 1.12.0
"@ | Set-Content -LiteralPath (Join-Path $output "$identifier.locale.zh-CN.yaml") -Encoding utf8NoBOM

Write-Host "WinGet manifests: $output"
