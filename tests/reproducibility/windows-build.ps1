param(
    [ValidateSet('x64', 'arm64')]
    [string]$Architecture = 'x64',
    [string]$EvidencePath
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
    foreach ($artifactName in $artifactNames) {
        $artifactPath = Join-Path $releaseDirectory $artifactName
        if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
            throw "Expected build artifact is missing: $artifactPath"
        }
        $hashes[$artifactName] = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    return $hashes
}

try {
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    $first = Invoke-IsolatedBuild -Label 'build-a'
    $second = Invoke-IsolatedBuild -Label 'build-b'

    $comparisons = foreach ($artifactName in $artifactNames) {
        [ordered]@{
            artifact = $artifactName
            build_a_sha256 = $first[$artifactName]
            build_b_sha256 = $second[$artifactName]
            matches = $first[$artifactName] -eq $second[$artifactName]
        }
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
        schema_version = 1
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
    $evidence | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
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
