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

try {
    New-Item -ItemType Directory -Path $fixture -Force | Out-Null
    $version = '9.8.7'
    $x64 = Join-Path $fixture 'ZiFile-9.8.7.0-windows-x64.msix'
    $arm64 = Join-Path $fixture 'ZiFile-9.8.7.0-windows-arm64.msix'
    [IO.File]::WriteAllText($x64, 'deterministic x64 official WinGet validation fixture')
    [IO.File]::WriteAllText($arm64, 'deterministic arm64 official WinGet validation fixture')
    & $generator `
        -Version $version `
        -X64InstallerUrl "https://github.com/ax2/zifile/releases/download/v$version/$(Split-Path $x64 -Leaf)" `
        -X64InstallerSha256 (Get-FileHash -LiteralPath $x64 -Algorithm SHA256).Hash `
        -Arm64InstallerUrl "https://github.com/ax2/zifile/releases/download/v$version/$(Split-Path $arm64 -Leaf)" `
        -Arm64InstallerSha256 (Get-FileHash -LiteralPath $arm64 -Algorithm SHA256).Hash `
        -OutputRoot $fixture | Out-Null
    $manifestDirectory = [IO.Path]::Combine([string[]]@(
        $fixture, 'manifests', 'z', 'ZiCode', 'ZiFile', $version
    ))
    $preflight = & $verifier `
        -ManifestDirectory $manifestDirectory `
        -Version $version `
        -X64InstallerPath $x64 `
        -Arm64InstallerPath $arm64 | ConvertFrom-Json
    if (-not $preflight.ready_for_winget_validate -or -not $preflight.local_installers_verified) {
        throw 'ZiFile preflight did not accept the deterministic WinGet candidate.'
    }
    if (@($preflight.file_extensions).Count -ne 29 -or
        $preflight.file_extensions -cnotcontains 'rar' -or
        $preflight.file_extensions -cnotcontains 'cab' -or
        $preflight.file_extensions -cnotcontains 'zipx' -or
        $preflight.file_extensions -cnotcontains 'cbr') {
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
            -X64InstallerPath $x64 `
            -Arm64InstallerPath $arm64
    }
    catch {
        if ($_.Exception.Message -notmatch 'file extensions do not match') { throw }
        $metadataDriftRejected = $true
    }
    if (-not $metadataDriftRejected) {
        throw 'ZiFile preflight accepted WinGet metadata with a missing RAR extension.'
    }

    [pscustomobject]@{
        schema_version = 1
        winget_version = $wingetVersion
        manifest_version = $preflight.manifest_version
        manifest_files = $preflight.manifest_files
        architectures = $preflight.architectures
        file_extensions = $preflight.file_extensions
        local_installers_verified = $preflight.local_installers_verified
        official_manifest_validation_passed = $true
        metadata_drift_rejected = $metadataDriftRejected
        public_assets_downloaded = $false
        community_repository_accepted = $false
    } | ConvertTo-Json -Depth 4
}
finally {
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
