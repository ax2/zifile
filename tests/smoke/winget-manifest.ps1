$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$generator = Join-Path $repoRoot 'packaging\winget\Generate-Manifests.ps1'
$verifier = Join-Path $repoRoot 'packaging\winget\Test-Manifests.ps1'
$winget = Get-Command winget.exe -ErrorAction SilentlyContinue
if (-not $winget) {
    throw 'winget.exe is required for the official manifest validation smoke test.'
}

$temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$fixture = [IO.Path]::GetFullPath((Join-Path $temporaryRoot (
    'zifile-winget-official-{0}' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $fixture.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the WinGet validation fixture outside the system temporary directory.'
}

function New-TestMsixBundle {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$PackageIdentity,
        [Parameter(Mandatory)][string]$PackageVersion
    )

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $innerPackages = @()
    try {
        foreach ($architecture in @('x64', 'arm64')) {
            $innerPath = Join-Path $fixture "ZiFile-$architecture.msix"
            $innerPackages += $innerPath
            $innerArchive = [IO.Compression.ZipFile]::Open(
                $innerPath,
                ([IO.Compression.ZipArchiveMode]::Create)
            )
            try {
                $entry = $innerArchive.CreateEntry('AppxManifest.xml')
                $writer = New-Object IO.StreamWriter($entry.Open())
                try {
                    $writer.Write(
                        "<Package xmlns='http://schemas.microsoft.com/appx/manifest/foundation/windows10'><Identity Name='$PackageIdentity' Publisher='CN=ZiCode Test' Version='$PackageVersion' /></Package>"
                    )
                }
                finally { $writer.Dispose() }
            }
            finally { $innerArchive.Dispose() }
        }

        $outerArchive = [IO.Compression.ZipFile]::Open(
            $Path,
            ([IO.Compression.ZipArchiveMode]::Create)
        )
        try {
            foreach ($architecture in @('x64', 'arm64')) {
                $innerPath = Join-Path $fixture "ZiFile-$architecture.msix"
                [IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                    $outerArchive,
                    $innerPath,
                    "ZiFile-9.8.7.0-windows-$architecture.msix",
                    ([IO.Compression.CompressionLevel]::Optimal)
                ) | Out-Null
            }
        }
        finally { $outerArchive.Dispose() }
    }
    finally {
        foreach ($innerPath in $innerPackages) {
            if (Test-Path -LiteralPath $innerPath) {
                Remove-Item -LiteralPath $innerPath -Force
            }
        }
    }
}

try {
    New-Item -ItemType Directory -Path $fixture -Force | Out-Null
    $version = '9.8.7'
    $bundle = Join-Path $fixture 'ZiFile-9.8.7.0-windows.msixbundle'
    New-TestMsixBundle -Path $bundle -PackageIdentity 'ZiCode.ZiFile' -PackageVersion '9.8.7.0'
    & $generator `
        -Version $version `
        -BundleInstallerUrl "https://github.com/ax2/zifile/releases/download/v$version/$(Split-Path $bundle -Leaf)" `
        -BundleInstallerSha256 (Get-FileHash -LiteralPath $bundle -Algorithm SHA256).Hash `
        -OutputRoot $fixture | Out-Null
    $manifestDirectory = [IO.Path]::Combine([string[]]@(
        $fixture, 'manifests', 'z', 'ZiCode', 'ZiFile', $version
    ))
    $preflight = & $verifier `
        -ManifestDirectory $manifestDirectory `
        -Version $version `
        -BundleInstallerPath $bundle | ConvertFrom-Json
    if (-not $preflight.ready_for_winget_validate -or
        -not $preflight.local_bundle_verified -or
        -not $preflight.package_identity_verified -or
        @($preflight.package_identity_names).Count -ne 1 -or
        $preflight.package_identity_names[0] -cne 'ZiCode.ZiFile' -or
        $preflight.public_installer_model -cne 'all-in-one-msixbundle') {
        throw 'ZiFile preflight did not accept the deterministic WinGet candidate.'
    }
    if (@($preflight.file_extensions).Count -ne 31 -or
        $preflight.file_extensions -cnotcontains 'rar' -or
        $preflight.file_extensions -cnotcontains 'cab' -or
        $preflight.file_extensions -cnotcontains 'zipx' -or
        $preflight.file_extensions -cnotcontains 'cbr' -or
        $preflight.file_extensions -cnotcontains 'tar.lz4' -or
        $preflight.file_extensions -cnotcontains 'tlz4') {
        throw 'ZiFile preflight did not preserve the complete open-extension contract.'
    }

    $wingetVersion = (& $winget.Source --version | Out-String).Trim()
    $validationOutput = & $winget.Source validate `
        --manifest $manifestDirectory `
        --disable-interactivity 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0 -or $validationOutput -notmatch 'Manifest validation succeeded') {
        throw "Official winget $wingetVersion validate rejected the generated candidate: $validationOutput"
    }

    $installerManifest = Join-Path $manifestDirectory 'ZiCode.ZiFile.installer.yaml'
    $installerSource = Get-Content -Raw -LiteralPath $installerManifest
    $driftedSource = $installerSource -replace '(?m)^- rar\r?\n', ''
    if ($driftedSource -ceq $installerSource) {
        throw 'Could not create the WinGet file-extension drift fixture.'
    }
    Set-Content -LiteralPath $installerManifest -Value $driftedSource -Encoding utf8NoBOM
    $metadataDriftRejected = $false
    try {
        $null = & $verifier `
            -ManifestDirectory $manifestDirectory `
            -Version $version `
            -BundleInstallerPath $bundle
    }
    catch {
        if ($_.Exception.Message -notmatch 'file extensions do not match') { throw }
        $metadataDriftRejected = $true
    }
    if (-not $metadataDriftRejected) {
        throw 'ZiFile preflight accepted WinGet metadata with a missing RAR extension.'
    }

    $developmentBundle = Join-Path $fixture 'ZiFile-9.8.7.0-windows-dev.msixbundle'
    New-TestMsixBundle -Path $developmentBundle -PackageIdentity 'ZiCode.ZiFile.Dev' -PackageVersion '9.8.7.0'
    $developmentBundleSha = (Get-FileHash -LiteralPath $developmentBundle -Algorithm SHA256).Hash
    Set-Content -LiteralPath $installerManifest `
        -Value ($installerSource -replace '(?m)(?<=InstallerSha256: )[A-F0-9]{64}', $developmentBundleSha) `
        -Encoding utf8NoBOM
    $developmentBundleRejected = $false
    try {
        $null = & $verifier `
            -ManifestDirectory $manifestDirectory `
            -Version $version `
            -BundleInstallerPath $developmentBundle
    }
    catch {
        if ($_.Exception.Message -notmatch 'Identity.Name') { throw }
        $developmentBundleRejected = $true
    }
    if (-not $developmentBundleRejected) {
        throw 'ZiFile preflight accepted a development MSIX package identity.'
    }
    $developmentEvidence = & $verifier `
        -ManifestDirectory $manifestDirectory `
        -Version $version `
        -BundleInstallerPath $developmentBundle `
        -AllowDevelopmentIdentity | ConvertFrom-Json
    if (-not $developmentEvidence.package_identity_verified -or
        $developmentEvidence.package_identity_expected -cne 'ZiCode.ZiFile.Dev') {
        throw 'ZiFile preflight did not allow the explicit development identity exception for GitHub evidence.'
    }

    [pscustomobject]@{
        schema_version = 1
        winget_version = $wingetVersion
        manifest_version = $preflight.manifest_version
        manifest_files = $preflight.manifest_files
        architectures = $preflight.architectures
        file_extensions = $preflight.file_extensions
        public_installer_model = $preflight.public_installer_model
        local_bundle_verified = $preflight.local_bundle_verified
        official_manifest_validation_passed = $true
        metadata_drift_rejected = $metadataDriftRejected
        development_identity_rejected = $developmentBundleRejected
        development_identity_allowed_for_evidence = $true
        public_assets_downloaded = $false
        community_repository_accepted = $false
    } | ConvertTo-Json -Depth 4
}
finally {
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
