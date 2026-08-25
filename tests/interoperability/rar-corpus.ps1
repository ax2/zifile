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
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-rar-corpus-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the RAR corpus outside the system temporary directory.'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot 'target\rar-corpus.json'
}
$EvidencePath = [IO.Path]::GetFullPath($EvidencePath)

function Resolve-SevenZip {
    if (-not [string]::IsNullOrWhiteSpace($SevenZipPath)) {
        $resolved = [IO.Path]::GetFullPath($SevenZipPath)
        if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
            throw "Configured 7-Zip executable does not exist: $resolved"
        }
        return $resolved
    }
    $command = Get-Command 7z.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) { return $command.Source }
    $candidate = Join-Path $env:ProgramFiles '7-Zip\7z.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    throw '7z.exe was not found. GitHub Windows Runner images are expected to provide 7-Zip.'
}

function Assert-TreesMatch {
    param(
        [Parameter(Mandatory)][string]$ExpectedRoot,
        [Parameter(Mandatory)][string]$ActualRoot
    )
    $expected = @(Get-ChildItem -LiteralPath $ExpectedRoot -Recurse -File | ForEach-Object {
        [IO.Path]::GetRelativePath($ExpectedRoot, $_.FullName)
    } | Sort-Object)
    $actual = @(Get-ChildItem -LiteralPath $ActualRoot -Recurse -File | ForEach-Object {
        [IO.Path]::GetRelativePath($ActualRoot, $_.FullName)
    } | Sort-Object)
    if (($expected -join "`n") -cne ($actual -join "`n")) {
        throw "RAR extracted file sets differ. Expected: $($expected -join ', '); actual: $($actual -join ', ')"
    }
    foreach ($relative in $expected) {
        $expectedHash = (Get-FileHash -LiteralPath (Join-Path $ExpectedRoot $relative) -Algorithm SHA256).Hash
        $actualHash = (Get-FileHash -LiteralPath (Join-Path $ActualRoot $relative) -Algorithm SHA256).Hash
        if ($expectedHash -cne $actualHash) { throw "RAR extracted content differs for $relative." }
    }
    return $expected.Count
}

$sourceCommit = '7d8f9386ef777a2415da34fe1db193d8471ff7d0'
$sourceRoot = "https://raw.githubusercontent.com/bitplane/rars/$sourceCommit/tests/fixtures"
$cases = @(
    [ordered]@{ name = 'rar13-compressed'; path = 'rar13/README.RAR'; sha256 = 'E5692692645C18BE15326273997FBEE0FB95CCCAF13A93F7557BB8469D44C23A'; password = $null },
    [ordered]@{ name = 'rar154-multifile'; path = 'rar15_40/rar154/doc_154_best.rar'; sha256 = 'FAA2B922D3AC7AE5BB4D7660E2B7DA5169AB79295574F1BA63BD72B59D2C407A'; password = $null },
    [ordered]@{ name = 'rar3-ppmd'; path = 'rar15_40/ppmd/ppmd_lorem_rar300.rar'; sha256 = '2C263BF552DE74D0A4D36142AE83FE44563A6FC18D1910B0FFCCE3958AA24574'; password = $null },
    [ordered]@{ name = 'rar5-default'; path = 'rar50/m3_default.rar'; sha256 = '3029E7D1A03E9AFAF9D5384CAA929CF1FE10165E8B15E38A6E509364A300AD59'; password = $null },
    [ordered]@{ name = 'rar5-e8e9-filter'; path = 'rar50/filter_e8e9.rar'; sha256 = 'EE93973EAC8A3CEF96982050CEF5FCE8A182780EFA3ED06063BAE62938C0F01E'; password = $null },
    [ordered]@{ name = 'winrar721-encrypted-quickopen'; path = 'rar50/winrar721_header_encrypted_quickopen.rar'; sha256 = '09004845CE334B61E75C49E12FB5412134E6C98BEBFC4C54D46E0BD27B912F4E'; password = 'Password' }
)
$rejectedCases = @(
    [ordered]@{ name = 'rar5-symlink'; path = 'rar50/wild/symlink.rar'; sha256 = 'A7336968564786EA5FF106FFF2F6EF8DFD8AD5ECCA86FBD1DB3D63BA42BCD29A' },
    [ordered]@{ name = 'rar5-hardlink'; path = 'rar50/wild/hardlink.rar'; sha256 = '2A0494AFAD63DCF7A5522AFD593BDE0CB42F53744888397FEBF112C00853F3D0' },
    [ordered]@{ name = 'rar5-redirection-hardlink'; path = 'rar50/wild/rarfile_hlink.rar'; sha256 = 'C7C2DB6FB021E89198DA80D4B903D4A46B692C2F31FEC64CB53C7E37AF7A51F4' }
)

Push-Location $repoRoot
try {
    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    if (-not $SkipBuild) {
        cargo build --workspace --locked
        if ($LASTEXITCODE -ne 0) { throw 'Workspace build failed.' }
    }
    $cli = Join-Path $repoRoot 'target\debug\zifile.exe'
    if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) { throw 'ZiFile CLI executable was not found.' }
    $sevenZip = Resolve-SevenZip
    $versionText = (& $sevenZip i) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'Could not query the 7-Zip version.' }
    $versionMatch = [regex]::Match($versionText, '7-Zip\s+([0-9]+(?:\.[0-9]+)+)')
    $sevenZipVersion = if ($versionMatch.Success) { $versionMatch.Groups[1].Value } else { 'unknown' }
    $results = @()

    foreach ($case in $cases) {
        $archive = Join-Path $testRoot ($case.name + '.rar')
        Invoke-WebRequest -Uri "$sourceRoot/$($case.path)" -OutFile $archive
        $actualArchiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
        if ($actualArchiveHash -cne $case.sha256) { throw "RAR fixture hash mismatch: $($case.name)" }

        if ($null -ne $case.password) {
            $case.password | & $cli test $archive --password-stdin | Out-Null
        } else {
            & $cli test $archive | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not test RAR case $($case.name)." }

        $zifileOutput = Join-Path $testRoot ($case.name + '-zifile')
        if ($null -ne $case.password) {
            $case.password | & $cli extract $archive $zifileOutput --password-stdin | Out-Null
        } else {
            & $cli extract $archive $zifileOutput | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not extract RAR case $($case.name)." }

        $sevenZipOutput = Join-Path $testRoot ($case.name + '-7zip')
        $sevenZipArguments = @('x', $archive, "-o$sevenZipOutput", '-y')
        if ($null -ne $case.password) { $sevenZipArguments += "-p$($case.password)" }
        & $sevenZip @sevenZipArguments | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "7-Zip could not extract RAR case $($case.name)." }
        $count = Assert-TreesMatch -ExpectedRoot $sevenZipOutput -ActualRoot $zifileOutput
        $results += [ordered]@{
            name = $case.name
            source_path = $case.path
            encrypted = $null -ne $case.password
            files = $count
            archive_bytes = (Get-Item -LiteralPath $archive).Length
            archive_sha256 = $actualArchiveHash
        }
    }

    foreach ($case in $rejectedCases) {
        $archive = Join-Path $testRoot ($case.name + '.rar')
        Invoke-WebRequest -Uri "$sourceRoot/$($case.path)" -OutFile $archive
        $actualArchiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
        if ($actualArchiveHash -cne $case.sha256) { throw "RAR rejection fixture hash mismatch: $($case.name)" }
        $rejectedOutput = Join-Path $testRoot ($case.name + '-rejected')
        & $cli extract $archive $rejectedOutput 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) { throw "ZiFile accepted unsafe RAR case $($case.name)." }
        if ((Test-Path -LiteralPath $rejectedOutput) -and @(Get-ChildItem -LiteralPath $rejectedOutput -Recurse -File).Count -ne 0) {
            throw "ZiFile wrote output for unsafe RAR case $($case.name)."
        }
        $results += [ordered]@{
            name = $case.name
            source_path = $case.path
            expected_rejection = 'link-or-redirection'
            archive_bytes = (Get-Item -LiteralPath $archive).Length
            archive_sha256 = $actualArchiveHash
        }
    }

    $evidence = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTime]::UtcNow.ToString('o')
        source_repository = 'https://github.com/bitplane/rars'
        source_commit = $sourceCommit
        seven_zip_version = $sevenZipVersion
        cases = $results
        passed = $true
    }
    $evidenceDirectory = Split-Path -Parent $EvidencePath
    if (-not (Test-Path -LiteralPath $evidenceDirectory)) {
        New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
    }
    $evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
    $evidence | ConvertTo-Json -Depth 6
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $testRoot) {
        $resolved = [IO.Path]::GetFullPath($testRoot)
        if ($resolved.StartsWith($tempRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolved -Recurse -Force
        }
    }
}
