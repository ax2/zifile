param(
    [string]$RepositoryRoot = (Split-Path $PSScriptRoot -Parent),
    [switch]$KeepDist
)

$ErrorActionPreference = 'Stop'
$root = [System.IO.Path]::GetFullPath($RepositoryRoot)
$rootMarker = Join-Path $root '.git'
if (-not (Test-Path -LiteralPath $rootMarker)) {
    throw "Refusing to clean a directory that is not a ZiFile repository: $root"
}

$relativeTargets = [System.Collections.Generic.List[string]]::new()
$relativeTargets.Add('target')
$relativeTargets.Add('docs\node_modules')
$relativeTargets.Add('docs\dist')
$relativeTargets.Add('docs\.astro')
$relativeTargets.Add('docs\.starlight')
$relativeTargets.Add('tests\helpers\msix-repair\bin')
$relativeTargets.Add('tests\helpers\msix-repair\obj')
if (-not $KeepDist) {
    $relativeTargets.Add('dist')
}

$rootWithSeparator = $root.TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
$removed = [System.Collections.Generic.List[string]]::new()
foreach ($relativeTarget in $relativeTargets) {
    $target = [IO.Path]::GetFullPath((Join-Path $root $relativeTarget))
    if (-not $target.StartsWith($rootWithSeparator, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clean a path outside the repository: $target"
    }
    if (Test-Path -LiteralPath $target) {
        Remove-Item -LiteralPath $target -Recurse -Force
        $removed.Add($relativeTarget)
    }
}

[pscustomobject]@{
    repository = $root
    keep_dist = [bool]$KeepDist
    removed = @($removed)
    removed_count = $removed.Count
} | ConvertTo-Json -Compress

