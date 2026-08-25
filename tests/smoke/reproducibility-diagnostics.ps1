$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repository = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$diagnosticScript = Join-Path $repository 'tests\reproducibility\windows-build.ps1'
$defaultDesktop = Join-Path $repository 'target\debug\zifile-desktop.exe'
$differentPe = Join-Path $repository 'target\debug\zifile-worker.exe'

foreach ($path in @($diagnosticScript, $defaultDesktop, $differentPe)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required reproducibility diagnostic input is missing: $path"
    }
}

$same = & $diagnosticScript `
    -ComparePeFirstPath $defaultDesktop `
    -ComparePeSecondPath $defaultDesktop |
    ConvertFrom-Json
if ($null -ne $same.first_difference_offset) {
    throw 'Identical PE files reported a first difference.'
}
if (@($same.pe_components | Where-Object { -not $_.matches }).Count -ne 0) {
    throw 'Identical PE files reported a mismatched component.'
}

$different = & $diagnosticScript `
    -ComparePeFirstPath $defaultDesktop `
    -ComparePeSecondPath $differentPe |
    ConvertFrom-Json
$differentComponents = @($different.pe_components | Where-Object { -not $_.matches })
if ($null -eq $different.first_difference_offset) {
    throw 'Different PE files did not report a first difference.'
}
if ($differentComponents.Count -eq 0) {
    throw 'Different PE files did not report a mismatched component.'
}
if (
    [string]::IsNullOrWhiteSpace($different.first_difference_component) -or
    $null -eq $different.first_difference_context -or
    [string]::IsNullOrWhiteSpace($different.first_difference_context.build_a_hex) -or
    [string]::IsNullOrWhiteSpace($different.first_difference_context.build_b_hex)
) {
    throw 'Different PE files did not preserve bounded first-difference context.'
}
$differentComponentWithContext = @(
    $differentComponents |
        Where-Object {
            $null -ne $_.first_difference_offset -and
            $null -ne $_.first_difference_context -and
            -not [string]::IsNullOrWhiteSpace($_.first_difference_context.build_a_hex) -and
            -not [string]::IsNullOrWhiteSpace($_.first_difference_context.build_b_hex)
        }
)
if ($differentComponentWithContext.Count -eq 0) {
    throw 'Different PE files did not preserve component-level difference context.'
}

[ordered]@{
    schema_version = 1
    identical_components = @($same.pe_components).Count
    identical_all_match = $true
    different_first_offset_found = $true
    different_context_preserved = $true
    different_component_context_preserved = $true
    different_components = $differentComponents.Count
} | ConvertTo-Json
