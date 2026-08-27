[CmdletBinding()]
param(
    [string]$ManifestPath = (Join-Path $PSScriptRoot 'listing-assets.json'),
    [string]$GeneratorPath = (Join-Path $PSScriptRoot '..\msix\Generate-Assets.ps1'),
    [switch]$VerifyGenerator
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$manifestFile = [IO.Path]::GetFullPath($ManifestPath)
if (-not (Test-Path -LiteralPath $manifestFile -PathType Leaf)) {
    throw "Store listing asset manifest does not exist: $manifestFile"
}
$manifest = Get-Content -Raw -LiteralPath $manifestFile | ConvertFrom-Json
if ($manifest.schema_version -ne 1 -or $manifest.status -cne 'ready') {
    throw 'Store listing asset manifest must be schema 1 and ready.'
}
if ($manifest.requirements_source -cne 'https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/screenshots-and-images') {
    throw 'Store listing asset manifest must retain the authoritative Microsoft requirements source.'
}
if (@($manifest.assets).Count -ne 1) {
    throw 'Store listing asset manifest must contain exactly one reviewed app tile icon.'
}

$asset = $manifest.assets[0]
$expectedHash = 'C1805A4271701152D5B1F043070625575C48F2926E474B5E2568C8288019F702'
if ($asset.kind -cne 'app_tile_icon' -or
    $asset.path -cne 'listing-assets/AppTile300x300.png' -or
    $asset.width -ne 300 -or $asset.height -ne 300 -or
    $asset.sha256 -cne $expectedHash) {
    throw 'Store app tile manifest metadata differs from the reviewed 300x300 asset contract.'
}

$manifestDirectory = Split-Path -Parent $manifestFile
$assetPath = [IO.Path]::GetFullPath((Join-Path $manifestDirectory ($asset.path -replace '/', [IO.Path]::DirectorySeparatorChar)))
if (-not $assetPath.StartsWith($manifestDirectory, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Store listing asset path resolves outside its manifest directory.'
}
if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) {
    throw "Store app tile asset does not exist: $($asset.path)"
}
$assetInfo = Get-Item -LiteralPath $assetPath
if ($assetInfo.Length -gt 50MB) {
    throw 'Store app tile asset exceeds the Microsoft Store 50 MB image limit.'
}

Add-Type -AssemblyName System.Drawing
$bitmap = [Drawing.Bitmap]::new($assetPath)
try {
    if ($bitmap.RawFormat.Guid -ne [Drawing.Imaging.ImageFormat]::Png.Guid) {
        throw 'Store app tile asset must be a PNG image.'
    }
    if ($bitmap.Width -ne 300 -or $bitmap.Height -ne 300) {
        throw "Store app tile asset must be 300x300, found $($bitmap.Width)x$($bitmap.Height)."
    }
    if (-not [Drawing.Image]::IsAlphaPixelFormat($bitmap.PixelFormat)) {
        throw 'Store app tile asset must retain an alpha-capable pixel format.'
    }
}
finally {
    $bitmap.Dispose()
}
$actualHash = (Get-FileHash -LiteralPath $assetPath -Algorithm SHA256).Hash
if ($actualHash -cne $expectedHash) {
    throw 'Store app tile asset hash does not match its reviewed pinned value.'
}

$generatorMatches = $null
if ($VerifyGenerator) {
    $generator = [IO.Path]::GetFullPath($GeneratorPath)
    if (-not (Test-Path -LiteralPath $generator -PathType Leaf)) {
        throw "Store listing asset generator does not exist: $generator"
    }
    $temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $fixture = [IO.Path]::GetFullPath((Join-Path $temporaryBase "zifile-store-icon-$([Guid]::NewGuid().ToString('N'))"))
    if (-not $fixture.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to create the Store icon fixture outside the system temporary directory.'
    }
    try {
        $packageOutput = Join-Path $fixture 'package-assets'
        $storeOutput = Join-Path $fixture 'store-assets'
        & $generator -OutputDirectory $packageOutput -StoreOutputDirectory $storeOutput | Out-Null
        $generatedHash = (Get-FileHash -LiteralPath (Join-Path $storeOutput 'AppTile300x300.png') -Algorithm SHA256).Hash
        if ($generatedHash -cne $expectedHash) {
            throw 'Store app tile differs from generator output on the current x64 maintenance host.'
        }
        $generatorMatches = $true
    }
    finally {
        if (Test-Path -LiteralPath $fixture) {
            $resolvedFixture = [IO.Path]::GetFullPath($fixture)
            if (-not $resolvedFixture.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase) -or
                [IO.Path]::GetFileName($resolvedFixture) -notlike 'zifile-store-icon-*') {
                throw "Refusing to remove unexpected Store icon fixture: $resolvedFixture"
            }
            Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
        }
    }
}

[pscustomobject]@{
    schema_version = 1
    validated = $true
    status = $manifest.status
    kind = $asset.kind
    width = 300
    height = 300
    bytes = $assetInfo.Length
    sha256 = $actualHash
    generator_matches_on_current_host = $generatorMatches
} | ConvertTo-Json
