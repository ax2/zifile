param(
    [string]$RepositoryRoot = (Split-Path $PSScriptRoot -Parent | Split-Path -Parent)
)

$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($RepositoryRoot)
$temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$fixture = [IO.Path]::GetFullPath((Join-Path $temporaryRoot ("zifile-cleanup-" + [guid]::NewGuid().ToString('N'))))
if (-not $fixture.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the cleanup fixture outside the system temporary directory.'
}

try {
    $gitMarker = Join-Path $fixture '.git'
    $target = Join-Path $fixture 'target'
    $docsNodeModules = Join-Path $fixture 'docs\node_modules'
    $docsDist = Join-Path $fixture 'docs\dist'
    $docsAstro = Join-Path $fixture 'docs\.astro'
    $docsStarlight = Join-Path $fixture 'docs\.starlight'
    $helperBin = Join-Path $fixture 'tests\helpers\msix-repair\bin'
    $helperObj = Join-Path $fixture 'tests\helpers\msix-repair\obj'
    $dist = Join-Path $fixture 'dist'
    $source = Join-Path $fixture 'crates\zifile-core\src'
    foreach ($directory in @($gitMarker, $target, $docsNodeModules, $docsDist, $docsAstro, $docsStarlight, $helperBin, $helperObj, $dist, $source)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }

    $cleanup = Join-Path $root 'scripts\Clean-BuildArtifacts.ps1'
    $result = & $cleanup -RepositoryRoot $fixture | ConvertFrom-Json
    if ($result.removed_count -ne 8 -or $result.keep_dist) {
        throw 'The cleanup script did not report the expected default target set.'
    }
    foreach ($directory in @($target, $docsNodeModules, $docsDist, $docsAstro, $docsStarlight, $helperBin, $helperObj, $dist)) {
        if (Test-Path -LiteralPath $directory) {
            throw "Generated directory was not removed: $directory"
        }
    }
    if (-not (Test-Path -LiteralPath $source -PathType Container)) {
        throw 'The cleanup script removed or changed a source directory.'
    }

    foreach ($directory in @($target, $docsNodeModules, $docsDist, $docsAstro, $docsStarlight, $helperBin, $helperObj, $dist)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    $keepResult = & $cleanup -RepositoryRoot $fixture -KeepDist | ConvertFrom-Json
    if (-not $keepResult.keep_dist -or (Test-Path -LiteralPath $dist) -eq $false) {
        throw 'The cleanup script did not preserve dist when -KeepDist was requested.'
    }
    if (Test-Path -LiteralPath $target) {
        throw 'The cleanup script preserved target unexpectedly.'
    }

    [pscustomobject]@{
        default_removed = $result.removed_count
        keep_dist_preserved = $true
        source_preserved = $true
    } | ConvertTo-Json
}
finally {
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
