param(
    [Parameter(Mandatory)]
    [string]$ManifestDirectory,
    [Parameter(Mandatory)]
    [string]$Version,
    [string]$X64InstallerPath,
    [string]$Arm64InstallerPath
)

$ErrorActionPreference = 'Stop'
$identifier = 'ZiCode.ZiFile'
$manifestVersion = '1.12.0'
$expectedFileExtensions = @(
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
$directory = [IO.Path]::GetFullPath($ManifestDirectory)
$expectedSuffix = [IO.Path]::Combine([string[]]@('manifests', 'z', 'ZiCode', 'ZiFile', $Version))
if (-not $directory.EndsWith($expectedSuffix, [StringComparison]::Ordinal)) {
    throw "WinGet manifest directory must end with '$expectedSuffix'."
}
if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
    throw "WinGet manifest directory does not exist: $directory"
}

$expectedFiles = @(
    "$identifier.yaml",
    "$identifier.installer.yaml",
    "$identifier.locale.en-US.yaml",
    "$identifier.locale.zh-CN.yaml"
)
$actualFiles = @(Get-ChildItem -LiteralPath $directory -File -Filter '*.yaml' | Sort-Object Name)
if ($actualFiles.Count -ne $expectedFiles.Count -or
    @($actualFiles.Name | Where-Object { $_ -notin $expectedFiles }).Count -ne 0) {
    throw "WinGet candidate must contain exactly the four expected multi-file manifests: $($expectedFiles -join ', ')."
}

$types = @{
    "$identifier.yaml" = 'version'
    "$identifier.installer.yaml" = 'installer'
    "$identifier.locale.en-US.yaml" = 'defaultLocale'
    "$identifier.locale.zh-CN.yaml" = 'locale'
}
foreach ($fileName in $expectedFiles) {
    $path = Join-Path $directory $fileName
    $source = Get-Content -Raw -LiteralPath $path
    $type = $types[$fileName]
    $schema = "# yaml-language-server: `$schema=https://aka.ms/winget-manifest.$type.$manifestVersion.schema.json"
    if (-not $source.StartsWith($schema, [StringComparison]::Ordinal)) {
        throw "$fileName does not begin with the matching WinGet schema declaration."
    }
    foreach ($requiredLine in @(
        "PackageIdentifier: $identifier",
        "PackageVersion: $Version",
        "ManifestType: $type",
        "ManifestVersion: $manifestVersion"
    )) {
        if ($source -notmatch "(?m)^$([Regex]::Escape($requiredLine))\r?$") {
            throw "$fileName is missing required line '$requiredLine'."
        }
    }
}

$installerSource = Get-Content -Raw -LiteralPath (Join-Path $directory "$identifier.installer.yaml")
foreach ($requiredLine in @(
    'Platform:',
    '- Windows.Desktop',
    'MinimumOSVersion: 10.0.19041.0',
    'InstallerType: msix',
    'Scope: user',
    'UpgradeBehavior: install'
)) {
    if ($installerSource -notmatch "(?m)^$([Regex]::Escape($requiredLine))\r?$") {
        throw "Installer manifest is missing required line '$requiredLine'."
    }
}

$extensionBlock = [Regex]::Match(
    $installerSource,
    '(?ms)^FileExtensions:\r?\n(?<items>(?:- [a-z0-9.]+\r?\n)+)Installers:'
)
if (-not $extensionBlock.Success) {
    throw 'Installer manifest does not contain a bounded FileExtensions list before Installers.'
}
$actualFileExtensions = @(
    [Regex]::Matches($extensionBlock.Groups['items'].Value, '(?m)^- (?<extension>[a-z0-9.]+)\r?$') |
        ForEach-Object { $_.Groups['extension'].Value }
)
if ($actualFileExtensions.Count -ne $expectedFileExtensions.Count -or
    (Compare-Object -ReferenceObject $expectedFileExtensions -DifferenceObject $actualFileExtensions -SyncWindow 0)) {
    throw "Installer manifest file extensions do not match ZiFile's supported open extensions."
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$coreSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'crates\zifile-core\src\lib.rs')
$coreExtensionBlock = [Regex]::Match(
    $coreSource,
    '(?s)pub const OPEN_ARCHIVE_EXTENSIONS: &\[&str\] = &\[(?<items>.*?)\];'
)
if (-not $coreExtensionBlock.Success) {
    throw 'Could not locate the core OPEN_ARCHIVE_EXTENSIONS contract.'
}
$coreFileExtensions = @(
    [Regex]::Matches($coreExtensionBlock.Groups['items'].Value, '"(?<extension>[a-z0-9.]+)"') |
        ForEach-Object { $_.Groups['extension'].Value }
)
if ($coreFileExtensions.Count -ne $expectedFileExtensions.Count -or
    (Compare-Object -ReferenceObject $expectedFileExtensions -DifferenceObject $coreFileExtensions -SyncWindow 0)) {
    throw 'WinGet file extensions have drifted from the core OPEN_ARCHIVE_EXTENSIONS contract.'
}

$installerMatches = [Regex]::Matches(
    $installerSource,
    '(?ms)^- Architecture: (?<architecture>x64|arm64)\r?\n\s+InstallerUrl: (?<url>\S+)\r?\n\s+InstallerSha256: (?<sha>[A-F0-9]{64})\r?$'
)
if ($installerMatches.Count -ne 2 -or
    @($installerMatches | ForEach-Object { $_.Groups['architecture'].Value } | Sort-Object -Unique).Count -ne 2) {
    throw 'Installer manifest must contain exactly one x64 and one arm64 installer with uppercase SHA-256 values.'
}

$localPaths = @{ x64 = $X64InstallerPath; arm64 = $Arm64InstallerPath }
$validated = @()
foreach ($match in $installerMatches) {
    $architecture = $match.Groups['architecture'].Value
    $url = [uri]$match.Groups['url'].Value
    $sha = $match.Groups['sha'].Value
    $expectedPrefix = "https://github.com/ax2/zifile/releases/download/v$Version/"
    if ($url.Scheme -ne 'https' -or
        -not $url.AbsoluteUri.StartsWith($expectedPrefix, [StringComparison]::Ordinal) -or
        -not $url.AbsolutePath.EndsWith("windows-$architecture.msix", [StringComparison]::OrdinalIgnoreCase)) {
        throw "$architecture installer URL is not the matching versioned ZiFile GitHub Release asset."
    }
    $localPath = $localPaths[$architecture]
    if ($localPath) {
        $resolved = (Resolve-Path -LiteralPath $localPath -ErrorAction Stop).Path
        if (-not $resolved.EndsWith("windows-$architecture.msix", [StringComparison]::OrdinalIgnoreCase)) {
            throw "$architecture local installer name does not match its architecture."
        }
        $actualSha = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash
        if ($actualSha -ne $sha) {
            throw "$architecture manifest SHA-256 does not match the signed local MSIX."
        }
    }
    $validated += $architecture
}

[pscustomobject]@{
    schema_version = 1
    package_identifier = $identifier
    package_version = $Version
    manifest_version = $manifestVersion
    manifest_files = $expectedFiles.Count
    architectures = @($validated | Sort-Object)
    file_extensions = $actualFileExtensions
    local_installers_verified = [bool]($X64InstallerPath -and $Arm64InstallerPath)
    community_repository_path = $directory
    ready_for_winget_validate = $true
} | ConvertTo-Json -Depth 4
