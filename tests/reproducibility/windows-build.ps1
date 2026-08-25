param(
    [ValidateSet('x64', 'arm64')]
    [string]$Architecture = 'x64',
    [string]$EvidencePath,
    [string]$ComparePeFirstPath,
    [string]$ComparePeSecondPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repository = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$targetTriple = switch ($Architecture) {
    'x64' { 'x86_64-pc-windows-msvc' }
    'arm64' { 'aarch64-pc-windows-msvc' }
}
$artifactNames = @(
    'zifile-desktop.exe',
    'zifile-desktop-accessible.exe',
    'zifile.exe',
    'zifile-worker.exe',
    'zifile_shell.dll'
)

if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repository "target\reproducibility-$Architecture.json"
}
$EvidencePath = [IO.Path]::GetFullPath($EvidencePath)
$evidenceDirectory = Split-Path -Parent $EvidencePath
if (-not (Test-Path -LiteralPath $evidenceDirectory)) {
    New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
}

$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd(
    [IO.Path]::DirectorySeparatorChar,
    [IO.Path]::AltDirectorySeparatorChar
)
$temporaryRoot = Join-Path $temporaryBase "zifile-repro-$([Guid]::NewGuid().ToString('N'))"
$originalTargetDirectory = $env:CARGO_TARGET_DIR
$originalIncremental = $env:CARGO_INCREMENTAL
$originalRustFlags = $env:RUSTFLAGS

function Get-RangeSha256 {
    param(
        [Parameter(Mandatory)]
        [byte[]]$Bytes,
        [Parameter(Mandatory)]
        [int]$Offset,
        [Parameter(Mandatory)]
        [int]$Count
    )

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        return [Convert]::ToHexString(
            $algorithm.ComputeHash($Bytes, $Offset, $Count)
        ).ToLowerInvariant()
    } finally {
        $algorithm.Dispose()
    }
}

function Get-PeLayout {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 64 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) {
        throw "Expected a PE image with an MZ header: $Path"
    }
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
    if (
        $peOffset -lt 0 -or
        $peOffset + 24 -gt $bytes.Length -or
        $bytes[$peOffset] -ne 0x50 -or
        $bytes[$peOffset + 1] -ne 0x45 -or
        $bytes[$peOffset + 2] -ne 0 -or
        $bytes[$peOffset + 3] -ne 0
    ) {
        throw "Expected a valid PE signature: $Path"
    }

    $coffOffset = $peOffset + 4
    $sectionCount = [BitConverter]::ToUInt16($bytes, $coffOffset + 2)
    $optionalHeaderSize = [BitConverter]::ToUInt16($bytes, $coffOffset + 16)
    $sectionTableOffset = $coffOffset + 20 + $optionalHeaderSize
    $sectionTableEnd = $sectionTableOffset + (40 * $sectionCount)
    if ($sectionTableEnd -gt $bytes.Length) {
        throw "PE section table extends beyond the file: $Path"
    }

    $rawSections = @()
    for ($index = 0; $index -lt $sectionCount; $index++) {
        $entryOffset = $sectionTableOffset + (40 * $index)
        $name = [Text.Encoding]::ASCII.GetString($bytes, $entryOffset, 8).Trim([char]0)
        $rawSize = [int64][BitConverter]::ToUInt32($bytes, $entryOffset + 16)
        $rawOffset = [int64][BitConverter]::ToUInt32($bytes, $entryOffset + 20)
        if ($rawOffset + $rawSize -gt $bytes.Length) {
            throw "PE section $name extends beyond the file: $Path"
        }
        $rawSections += [pscustomobject]@{
            name = $name
            raw_offset = $rawOffset
            raw_size = $rawSize
        }
    }

    $firstRawOffset = @(
        $rawSections |
            Where-Object { $_.raw_size -gt 0 } |
            ForEach-Object { $_.raw_offset } |
            Sort-Object
    ) | Select-Object -First 1
    $headerSize = if ($null -eq $firstRawOffset) { $sectionTableEnd } else { $firstRawOffset }
    if ($headerSize -gt $bytes.Length) {
        throw "PE headers extend beyond the file: $Path"
    }

    $components = @(
        [pscustomobject]@{
            name = '<headers>'
            raw_offset = [int64]0
            raw_size = [int64]$headerSize
            sha256 = Get-RangeSha256 -Bytes $bytes -Offset 0 -Count ([int]$headerSize)
        }
    )
    foreach ($section in $rawSections) {
        $components += [pscustomobject]@{
            name = $section.name
            raw_offset = $section.raw_offset
            raw_size = $section.raw_size
            sha256 = Get-RangeSha256 `
                -Bytes $bytes `
                -Offset ([int]$section.raw_offset) `
                -Count ([int]$section.raw_size)
        }
    }

    $contentEnd = [int64]$headerSize
    foreach ($section in $rawSections) {
        $contentEnd = [Math]::Max($contentEnd, $section.raw_offset + $section.raw_size)
    }
    if ($contentEnd -lt $bytes.Length) {
        $overlaySize = [int64]$bytes.Length - $contentEnd
        $components += [pscustomobject]@{
            name = '<overlay>'
            raw_offset = $contentEnd
            raw_size = $overlaySize
            sha256 = Get-RangeSha256 `
                -Bytes $bytes `
                -Offset ([int]$contentEnd) `
                -Count ([int]$overlaySize)
        }
    }

    return [pscustomobject]@{
        bytes = $bytes
        components = @($components)
    }
}

function Find-FirstDifferenceInRange {
    param(
        [Parameter(Mandatory)]
        [byte[]]$First,
        [Parameter(Mandatory)]
        [byte[]]$Second,
        [Parameter(Mandatory)]
        [int]$Offset,
        [Parameter(Mandatory)]
        [int]$Count
    )

    $chunkSize = 64 * 1024
    for ($chunkOffset = 0; $chunkOffset -lt $Count; $chunkOffset += $chunkSize) {
        $currentCount = [Math]::Min($chunkSize, $Count - $chunkOffset)
        $absoluteOffset = $Offset + $chunkOffset
        $firstHash = Get-RangeSha256 -Bytes $First -Offset $absoluteOffset -Count $currentCount
        $secondHash = Get-RangeSha256 -Bytes $Second -Offset $absoluteOffset -Count $currentCount
        if ($firstHash -ne $secondHash) {
            for ($index = 0; $index -lt $currentCount; $index++) {
                if ($First[$absoluteOffset + $index] -ne $Second[$absoluteOffset + $index]) {
                    return [int64]($absoluteOffset + $index)
                }
            }
        }
    }
    return $null
}

function Compare-PeArtifacts {
    param(
        [Parameter(Mandatory)]
        [string]$FirstPath,
        [Parameter(Mandatory)]
        [string]$SecondPath
    )

    $first = Get-PeLayout -Path $FirstPath
    $second = Get-PeLayout -Path $SecondPath
    $componentCount = [Math]::Max($first.components.Count, $second.components.Count)
    $components = @()
    $firstDifference = $null
    $firstDifferenceComponent = $null
    $firstDifferenceComponentOffset = $null
    for ($index = 0; $index -lt $componentCount; $index++) {
        $firstComponent = if ($index -lt $first.components.Count) { $first.components[$index] } else { $null }
        $secondComponent = if ($index -lt $second.components.Count) { $second.components[$index] } else { $null }
        $sameLayout = $null -ne $firstComponent -and $null -ne $secondComponent -and
            $firstComponent.name -eq $secondComponent.name -and
            $firstComponent.raw_offset -eq $secondComponent.raw_offset -and
            $firstComponent.raw_size -eq $secondComponent.raw_size
        $matches = $sameLayout -and $firstComponent.sha256 -eq $secondComponent.sha256
        $components += [ordered]@{
            name_a = if ($null -eq $firstComponent) { $null } else { $firstComponent.name }
            name_b = if ($null -eq $secondComponent) { $null } else { $secondComponent.name }
            raw_offset_a = if ($null -eq $firstComponent) { $null } else { $firstComponent.raw_offset }
            raw_offset_b = if ($null -eq $secondComponent) { $null } else { $secondComponent.raw_offset }
            raw_size_a = if ($null -eq $firstComponent) { $null } else { $firstComponent.raw_size }
            raw_size_b = if ($null -eq $secondComponent) { $null } else { $secondComponent.raw_size }
            build_a_sha256 = if ($null -eq $firstComponent) { $null } else { $firstComponent.sha256 }
            build_b_sha256 = if ($null -eq $secondComponent) { $null } else { $secondComponent.sha256 }
            matches = $matches
        }
        if ($null -eq $firstDifference -and -not $matches) {
            $firstDifferenceComponent = if ($null -eq $firstComponent) {
                $secondComponent.name
            } else {
                $firstComponent.name
            }
            if ($sameLayout) {
                $firstDifference = Find-FirstDifferenceInRange `
                    -First $first.bytes `
                    -Second $second.bytes `
                    -Offset ([int]$firstComponent.raw_offset) `
                    -Count ([int]$firstComponent.raw_size)
                if ($null -ne $firstDifference) {
                    $firstDifferenceComponentOffset = $firstDifference - $firstComponent.raw_offset
                }
            } else {
                $candidateOffsets = @(
                    if ($null -ne $firstComponent) { $firstComponent.raw_offset }
                    if ($null -ne $secondComponent) { $secondComponent.raw_offset }
                )
                $firstDifference = ($candidateOffsets | Measure-Object -Minimum).Minimum
            }
        }
    }
    if ($null -eq $firstDifference -and $first.bytes.Length -ne $second.bytes.Length) {
        $firstDifference = [int64][Math]::Min($first.bytes.Length, $second.bytes.Length)
        $firstDifferenceComponent = '<file-length>'
        $firstDifferenceComponentOffset = 0
    }

    $context = $null
    if ($null -ne $firstDifference) {
        $contextStart = [Math]::Max([int64]0, $firstDifference - 16)
        $sharedLength = [Math]::Min($first.bytes.Length, $second.bytes.Length)
        $contextCount = [Math]::Min([int64]64, $sharedLength - $contextStart)
        if ($contextCount -gt 0) {
            $context = [ordered]@{
                start_offset = $contextStart
                byte_count = $contextCount
                build_a_hex = [Convert]::ToHexString(
                    $first.bytes,
                    [int]$contextStart,
                    [int]$contextCount
                ).ToLowerInvariant()
                build_b_hex = [Convert]::ToHexString(
                    $second.bytes,
                    [int]$contextStart,
                    [int]$contextCount
                ).ToLowerInvariant()
            }
        }
    }

    return [ordered]@{
        build_a_bytes = $first.bytes.Length
        build_b_bytes = $second.bytes.Length
        first_difference_offset = $firstDifference
        first_difference_component = $firstDifferenceComponent
        first_difference_component_offset = $firstDifferenceComponentOffset
        first_difference_context = $context
        pe_components = @($components)
    }
}

if (
    [string]::IsNullOrWhiteSpace($ComparePeFirstPath) -xor
    [string]::IsNullOrWhiteSpace($ComparePeSecondPath)
) {
    throw 'ComparePeFirstPath and ComparePeSecondPath must be supplied together.'
}
if (-not [string]::IsNullOrWhiteSpace($ComparePeFirstPath)) {
    Compare-PeArtifacts `
        -FirstPath ([IO.Path]::GetFullPath($ComparePeFirstPath)) `
        -SecondPath ([IO.Path]::GetFullPath($ComparePeSecondPath)) |
        ConvertTo-Json -Depth 8
    return
}

function Invoke-IsolatedBuild {
    param(
        [Parameter(Mandatory)]
        [string]$Label
    )

    $targetDirectory = Join-Path $temporaryRoot $Label
    $env:CARGO_TARGET_DIR = $targetDirectory
    $env:CARGO_INCREMENTAL = '0'
    $deterministicFlags = '-C link-arg=/Brepro'
    $env:RUSTFLAGS = if ([string]::IsNullOrWhiteSpace($originalRustFlags)) {
        $deterministicFlags
    } else {
        "$originalRustFlags $deterministicFlags"
    }

    Push-Location $repository
    try {
        & cargo build --workspace --release --locked --all-features --jobs 1 --target $targetTriple
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for isolated build $Label with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $releaseDirectory = Join-Path $targetDirectory "$targetTriple\release"
    $hashes = [ordered]@{}
    $paths = [ordered]@{}
    foreach ($artifactName in $artifactNames) {
        $artifactPath = Join-Path $releaseDirectory $artifactName
        if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
            throw "Expected build artifact is missing: $artifactPath"
        }
        $hashes[$artifactName] = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
        $paths[$artifactName] = $artifactPath
    }
    return [pscustomobject]@{
        hashes = $hashes
        paths = $paths
    }
}

try {
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    $first = Invoke-IsolatedBuild -Label 'build-a'
    $second = Invoke-IsolatedBuild -Label 'build-b'

    $comparisons = foreach ($artifactName in $artifactNames) {
        $matches = $first.hashes[$artifactName] -eq $second.hashes[$artifactName]
        $comparison = [ordered]@{
            artifact = $artifactName
            build_a_sha256 = $first.hashes[$artifactName]
            build_b_sha256 = $second.hashes[$artifactName]
            matches = $matches
        }
        if (-not $matches) {
            $comparison.diagnostics = Compare-PeArtifacts `
                -FirstPath $first.paths[$artifactName] `
                -SecondPath $second.paths[$artifactName]
        }
        $comparison
    }
    $allMatch = @($comparisons | Where-Object { -not $_.matches }).Count -eq 0
    $rustcVersion = (& rustc -Vv) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'rustc -Vv failed' }
    $cargoVersion = (& cargo -V) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'cargo -V failed' }
    $commit = (& git -C $repository rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'git rev-parse HEAD failed' }
    $dirty = -not [string]::IsNullOrWhiteSpace((& git -C $repository status --porcelain) -join "`n")

    $evidence = [ordered]@{
        schema_version = 2
        generated_at_utc = [DateTime]::UtcNow.ToString('o')
        source_commit = $commit
        source_dirty = $dirty
        architecture = $Architecture
        target_triple = $targetTriple
        command = "cargo build --workspace --release --locked --all-features --jobs 1 --target $targetTriple"
        cargo_incremental = 0
        cargo_build_jobs = 1
        deterministic_linker_flag = '/Brepro'
        rustc = $rustcVersion
        cargo = $cargoVersion
        builds = 2
        artifacts = @($comparisons)
        reproducible = $allMatch
    }
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
    Write-Host "Reproducibility evidence: $EvidencePath"
    foreach ($comparison in $comparisons) {
        Write-Host ("{0}: {1}" -f $comparison.artifact, $(if ($comparison.matches) { 'MATCH' } else { 'DIFFERENT' }))
    }
    if (-not $allMatch) {
        throw 'One or more release artifacts differ between isolated builds.'
    }
} finally {
    if ($null -eq $originalTargetDirectory) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $originalTargetDirectory
    }
    if ($null -eq $originalIncremental) {
        Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_INCREMENTAL = $originalIncremental
    }
    if ($null -eq $originalRustFlags) {
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    } else {
        $env:RUSTFLAGS = $originalRustFlags
    }

    $resolvedTemporaryRoot = [IO.Path]::GetFullPath($temporaryRoot)
    $requiredPrefix = $temporaryBase + [IO.Path]::DirectorySeparatorChar
    $safeLeaf = Split-Path -Leaf $resolvedTemporaryRoot
    if (
        $resolvedTemporaryRoot.StartsWith($requiredPrefix, [StringComparison]::OrdinalIgnoreCase) -and
        $safeLeaf.StartsWith('zifile-repro-', [StringComparison]::Ordinal)
    ) {
        if (Test-Path -LiteralPath $resolvedTemporaryRoot) {
            Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force
        }
    } else {
        throw "Refusing to clean unexpected temporary path: $resolvedTemporaryRoot"
    }
}
