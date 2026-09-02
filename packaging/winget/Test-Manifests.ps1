param(
    [Parameter(Mandatory)]
    [string]$ManifestDirectory,
    [Parameter(Mandatory)]
    [string]$Version,
    [string]$BundleInstallerPath,
    [switch]$AllowDevelopmentIdentity
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
    'bz', 'bz2', 'tar.bz2', 'tbz', 'tbz2', 'tar.lz4', 'tlz4',
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

$validated = @()
$bundleUrls = @()
$bundleHashes = @()
foreach ($match in $installerMatches) {
    $architecture = $match.Groups['architecture'].Value
    $url = [uri]$match.Groups['url'].Value
    $sha = $match.Groups['sha'].Value
    $expectedPrefix = "https://github.com/ax2/zifile/releases/download/v$Version/"
    if ($url.Scheme -ne 'https' -or
        -not $url.AbsoluteUri.StartsWith($expectedPrefix, [StringComparison]::Ordinal) -or
        -not $url.AbsolutePath.EndsWith('.msixbundle', [StringComparison]::OrdinalIgnoreCase)) {
        throw "$architecture installer URL is not the versioned ZiFile all-in-one MSIX bundle asset."
    }
    $bundleUrls += $url.AbsoluteUri
    $bundleHashes += $sha
    $validated += $architecture
}
if (@($bundleUrls | Sort-Object -Unique).Count -ne 1 -or
    @($bundleHashes | Sort-Object -Unique).Count -ne 1) {
    throw 'The x64 and arm64 WinGet entries must reference the same all-in-one MSIX bundle and SHA-256.'
}

$packageIdentities = @()
if ($BundleInstallerPath) {
    $resolvedBundle = (Resolve-Path -LiteralPath $BundleInstallerPath -ErrorAction Stop).Path
    if (-not $resolvedBundle.EndsWith('.msixbundle', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Local bundle installer name does not match the manifest URL.'
    }
    $actualBundleSha = (Get-FileHash -LiteralPath $resolvedBundle -Algorithm SHA256).Hash
    if ($actualBundleSha -ne $bundleHashes[0]) {
        throw 'The WinGet manifest SHA-256 does not match the local all-in-one MSIX bundle.'
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $outerArchive = [IO.Compression.ZipFile]::OpenRead($resolvedBundle)
    try {
        foreach ($architecture in @('x64', 'arm64')) {
            $msixEntry = @(
                $outerArchive.Entries |
                    Where-Object { $_.FullName -match "-windows-$architecture\.msix$" }
            )
            if ($msixEntry.Count -ne 1) {
                throw "MSIX bundle must contain exactly one $architecture package."
            }

            $innerStream = New-Object IO.MemoryStream
            try {
                $entryStream = $msixEntry[0].Open()
                try { $entryStream.CopyTo($innerStream) }
                finally { $entryStream.Dispose() }
                $innerStream.Position = 0
                $innerArchive = New-Object IO.Compression.ZipArchive(
                    $innerStream,
                    ([IO.Compression.ZipArchiveMode]::Read),
                    $false
                )
                try {
                    $manifestEntry = @(
                        $innerArchive.Entries |
                            Where-Object { $_.FullName -ieq 'AppxManifest.xml' }
                    )
                    if ($manifestEntry.Count -ne 1) {
                        throw "$architecture MSIX package does not contain AppxManifest.xml."
                    }
                    $reader = New-Object IO.StreamReader($manifestEntry[0].Open())
                    try { $manifestXml = [xml]$reader.ReadToEnd() }
                    finally { $reader.Dispose() }
                    $identity = $manifestXml.SelectSingleNode(
                        "/*[local-name()='Package']/*[local-name()='Identity']"
                    )
                    if ($null -eq $identity) {
                        throw "$architecture AppxManifest.xml does not contain Package/Identity."
                    }
                    $packageIdentities += [pscustomobject]@{
                        architecture = $architecture
                        name = $identity.GetAttribute('Name')
                        version = $identity.GetAttribute('Version')
                        publisher = $identity.GetAttribute('Publisher')
                    }
                }
                finally { $innerArchive.Dispose() }
            }
            finally { $innerStream.Dispose() }
        }
    }
    finally { $outerArchive.Dispose() }

    $expectedMsixVersion = "$Version.0"
    $expectedPackageIdentity = if ($AllowDevelopmentIdentity) {
        "$identifier.Dev"
    }
    else {
        $identifier
    }
    foreach ($packageIdentity in $packageIdentities) {
        if ($packageIdentity.name -ne $expectedPackageIdentity) {
            throw "$($packageIdentity.architecture) MSIX Identity.Name is '$($packageIdentity.name)', expected '$expectedPackageIdentity'. Development or renamed package identities cannot be submitted to WinGet."
        }
        if ($packageIdentity.version -ne $expectedMsixVersion) {
            throw "$($packageIdentity.architecture) MSIX Identity.Version is '$($packageIdentity.version)', expected '$expectedMsixVersion'."
        }
    }
}

[pscustomobject]@{
    schema_version = 1
    package_identifier = $identifier
    package_version = $Version
    manifest_version = $manifestVersion
    manifest_files = $expectedFiles.Count
    architectures = @($validated | Sort-Object)
    file_extensions = $actualFileExtensions
    public_installer_model = 'all-in-one-msixbundle'
    local_bundle_verified = [bool]$BundleInstallerPath
    local_installers_verified = [bool]$BundleInstallerPath
    package_identity_verified = ($packageIdentities.Count -eq 2)
    package_identity_expected = if ($BundleInstallerPath) { $expectedPackageIdentity } else { $null }
    package_identity_names = @($packageIdentities | Select-Object -ExpandProperty name -Unique)
    community_repository_path = $directory
    ready_for_winget_validate = $true
} | ConvertTo-Json -Depth 4
