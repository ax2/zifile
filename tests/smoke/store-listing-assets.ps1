$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$validator = Join-Path $repoRoot 'packaging\store\Test-ListingAssets.ps1'
$sourceRoot = Join-Path $repoRoot 'packaging\store'
$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$fixture = [IO.Path]::GetFullPath((Join-Path $temporaryBase "zifile-store-assets-$([Guid]::NewGuid().ToString('N'))"))
if (-not $fixture.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the Store asset fixture outside the system temporary directory.'
}

function Get-ExpectedFailure {
    param([Parameter(Mandatory)][scriptblock]$Action, [Parameter(Mandatory)][string]$Pattern)
    try { & $Action | Out-Null }
    catch {
        if ($_.Exception.Message -notmatch $Pattern) {
            throw "Expected failure matching '$Pattern', received: $($_.Exception.Message)"
        }
        return
    }
    throw "Expected action to fail with '$Pattern'."
}

try {
    $valid = & $validator | ConvertFrom-Json
    if (-not $valid.validated -or $valid.width -ne 300 -or $valid.height -ne 300) {
        throw 'Reviewed Store app tile did not pass validation.'
    }

    $fixtureAssets = Join-Path $fixture 'listing-assets'
    New-Item -ItemType Directory -Path $fixtureAssets -Force | Out-Null
    $fixtureManifest = Join-Path $fixture 'listing-assets.json'
    $fixtureIcon = Join-Path $fixtureAssets 'AppTile300x300.png'
    Copy-Item -LiteralPath (Join-Path $sourceRoot 'listing-assets.json') -Destination $fixtureManifest
    Copy-Item -LiteralPath (Join-Path $sourceRoot 'listing-assets\AppTile300x300.png') -Destination $fixtureIcon

    Remove-Item -LiteralPath $fixtureIcon
    Get-ExpectedFailure -Pattern 'does not exist' -Action { & $validator -ManifestPath $fixtureManifest }

    Copy-Item -LiteralPath (Join-Path $repoRoot 'packaging\msix\Assets\Square310x310Logo.png') -Destination $fixtureIcon
    Get-ExpectedFailure -Pattern 'must be 300x300' -Action { & $validator -ManifestPath $fixtureManifest }

    Copy-Item -LiteralPath (Join-Path $sourceRoot 'listing-assets\AppTile300x300.png') -Destination $fixtureIcon -Force
    Add-Type -AssemblyName System.Drawing
    $loaded = [Drawing.Bitmap]::new($fixtureIcon)
    try { $changed = [Drawing.Bitmap]::new($loaded) }
    finally { $loaded.Dispose() }
    try {
        $changed.SetPixel(0, 0, [Drawing.Color]::FromArgb(255, 255, 0, 0))
        $changed.Save($fixtureIcon, [Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $changed.Dispose() }
    Get-ExpectedFailure -Pattern 'hash does not match' -Action { & $validator -ManifestPath $fixtureManifest }

    [pscustomobject]@{
        schema_version = 1
        valid_store_icon_accepted = $true
        missing_store_icon_rejected = $true
        incorrect_store_icon_dimensions_rejected = $true
        modified_store_icon_rejected = $true
    } | ConvertTo-Json
}
finally {
    if (Test-Path -LiteralPath $fixture) {
        $resolvedFixture = [IO.Path]::GetFullPath($fixture)
        if (-not $resolvedFixture.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase) -or
            [IO.Path]::GetFileName($resolvedFixture) -notlike 'zifile-store-assets-*') {
            throw "Refusing to remove unexpected Store asset fixture: $resolvedFixture"
        }
        Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
    }
}
