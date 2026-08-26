param(
    [switch]$SkipBuild,
    [string]$SevenZipPath,
    [string]$EvidencePath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd(
    [IO.Path]::DirectorySeparatorChar,
    [IO.Path]::AltDirectorySeparatorChar
)
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-zip-legacy-corpus-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the ZIP legacy corpus outside the system temporary directory.'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot 'target\zip-legacy-corpus.json'
}
$EvidencePath = [IO.Path]::GetFullPath($EvidencePath)
$sourceCommit = '771dfc534d2614158af5497ea3dff4d4208d7db1'
$sourceBase = "https://raw.githubusercontent.com/zip-rs/zip2/$sourceCommit/tests/data"

function Resolve-SevenZip {
    if (-not [string]::IsNullOrWhiteSpace($SevenZipPath)) {
        $resolved = [IO.Path]::GetFullPath($SevenZipPath)
        if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
            throw "Configured 7-Zip executable does not exist: $resolved"
        }
        return $resolved
    }
    $command = Get-Command 7z.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }
    $programFilesCandidate = Join-Path $env:ProgramFiles '7-Zip\7z.exe'
    if (Test-Path -LiteralPath $programFilesCandidate -PathType Leaf) {
        return $programFilesCandidate
    }
    throw '7z.exe was not found. GitHub Windows Runner images are expected to provide 7-Zip.'
}

function Invoke-SevenZip {
    param([Parameter(Mandatory)][string[]]$Arguments)

    $output = @(& $script:sevenZip @Arguments)
    if ($LASTEXITCODE -ne 0) {
        throw "7-Zip failed with exit code $LASTEXITCODE."
    }
    return $output
}

function Get-Sha256 {
    param([Parameter(Mandatory)][string]$Path)

    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
}

function Receive-PinnedFile {
    param(
        [Parameter(Mandatory)][Net.Http.HttpClient]$Client,
        [Parameter(Mandatory)][string]$RelativePath,
        [Parameter(Mandatory)][string]$ExpectedSha256,
        [Parameter(Mandatory)][string]$Destination
    )

    $url = "$sourceBase/$RelativePath"
    $bytes = $Client.GetByteArrayAsync($url).GetAwaiter().GetResult()
    [IO.File]::WriteAllBytes($Destination, $bytes)
    $actual = Get-Sha256 -Path $Destination
    if ($actual -ne $ExpectedSha256) {
        throw "Pinned ZIP legacy corpus hash mismatch for $RelativePath."
    }
    return $bytes.Length
}

function Assert-SingleGoldenFile {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][string]$ExpectedName,
        [Parameter(Mandatory)][string]$GoldenSha256
    )

    $files = @(Get-ChildItem -LiteralPath $Root -Recurse -File)
    if ($files.Count -ne 1) {
        throw "Expected one extracted legacy ZIP file, found $($files.Count)."
    }
    $relative = [IO.Path]::GetRelativePath($Root, $files[0].FullName)
    if ($relative -cne $ExpectedName) {
        throw "Expected extracted path '$ExpectedName', found '$relative'."
    }
    if ((Get-Sha256 -Path $files[0].FullName) -ne $GoldenSha256) {
        throw "Extracted legacy ZIP content differs for $ExpectedName."
    }
    return $files.Count
}

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        cargo build --workspace --locked
        if ($LASTEXITCODE -ne 0) { throw 'Workspace build failed.' }
    }
    $cli = Join-Path $repoRoot 'target\debug\zifile.exe'
    if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) {
        throw 'ZiFile CLI executable was not found.'
    }
    $script:sevenZip = Resolve-SevenZip
    $versionText = (& $script:sevenZip i) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'Could not query the 7-Zip version.' }
    $versionMatch = [regex]::Match($versionText, '7-Zip(?: \(a\))?\s+([0-9]+(?:\.[0-9]+)+)')
    $sevenZipVersion = if ($versionMatch.Success) { $versionMatch.Groups[1].Value } else { 'unknown' }

    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    $goldenPath = Join-Path $testRoot 'first.txt'
    $goldenSha256 = '7FA9E80FCFC8EF32D3E08D88B85730803DA855AFFEA2D1EC51F08A4B01F171E7'
    $cases = @(
        [ordered]@{ name = 'shrink'; method = 'Shrink'; expected_entry = 'FIRST.TXT'; sha256 = '04D2B9534D3D0A07AE2FDA191A464B32BAE516A4B9471BE29120755431FADDF4'; archive = $null; bytes = 0 },
        [ordered]@{ name = 'reduce'; method = 'Reduce4'; expected_entry = 'first.txt'; sha256 = 'BD76C104ED775B189A1EBF25F1F5D7F4A1CFF42E01EF66D2AF570DDBA6F8D2F6'; archive = $null; bytes = 0 },
        [ordered]@{ name = 'implode'; method = 'Implode:v3'; expected_entry = 'first.txt'; sha256 = '36EBF1DC4833767728E1CABB99ABA83137931638A6B07754D437A3ADEFC7984A'; archive = $null; bytes = 0 }
    )

    $http = [Net.Http.HttpClient]::new()
    try {
        $goldenBytes = Receive-PinnedFile -Client $http -RelativePath 'folder/first.txt' -ExpectedSha256 $goldenSha256 -Destination $goldenPath
        foreach ($case in $cases) {
            $case.archive = Join-Path $testRoot ("$($case.name).zip")
            $case.bytes = Receive-PinnedFile -Client $http -RelativePath ("legacy/$($case.name).zip") -ExpectedSha256 $case.sha256 -Destination $case.archive
        }
    }
    finally {
        $http.Dispose()
    }

    $results = @()
    foreach ($case in $cases) {
        & $cli test $case.archive | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not test legacy ZIP case $($case.name)." }
        $zifileOutput = Join-Path $testRoot ("zifile-$($case.name)")
        & $cli extract $case.archive $zifileOutput | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not extract legacy ZIP case $($case.name)." }
        $zifileFiles = Assert-SingleGoldenFile -Root $zifileOutput -ExpectedName $case.expected_entry -GoldenSha256 $goldenSha256

        $listing = @(Invoke-SevenZip -Arguments @('l', '-slt', $case.archive))
        $reportedMethods = @($listing |
            Where-Object { $_ -match '^Method = (.+)$' } |
            ForEach-Object { $Matches[1] } |
            Sort-Object -Unique)
        if (($reportedMethods -join ' ') -notmatch [regex]::Escape($case.method)) {
            throw "7-Zip did not report expected method '$($case.method)' for $($case.name)."
        }
        $results += [ordered]@{
            name = $case.name
            method = $case.method
            expected_entry = $case.expected_entry
            archive_bytes = $case.bytes
            archive_sha256 = $case.sha256
            zifile_files = $zifileFiles
            seven_zip_reported_methods = $reportedMethods
            seven_zip_identified_method = $true
            passed = $true
        }
    }

    $evidence = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTime]::UtcNow.ToString('o')
        source_repository = 'zip-rs/zip2'
        source_commit = $sourceCommit
        golden_bytes = $goldenBytes
        golden_sha256 = $goldenSha256
        seven_zip_version = $sevenZipVersion
        cases = $results
        passed = $true
    }
    $evidenceDirectory = Split-Path -Parent $EvidencePath
    if (-not (Test-Path -LiteralPath $evidenceDirectory)) {
        New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
    }
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
    $evidence | ConvertTo-Json -Depth 8
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $testRoot) {
        $resolved = [IO.Path]::GetFullPath($testRoot)
        if ($resolved.StartsWith($tempRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            [IO.Directory]::Delete($resolved, $true)
        }
    }
}
