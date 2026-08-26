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
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-zip-method-corpus-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the ZIP method corpus outside the system temporary directory.'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot 'target\zip-method-corpus.json'
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

function Assert-FixtureMatches {
    param(
        [Parameter(Mandatory)][string]$ExpectedRoot,
        [Parameter(Mandatory)][string]$ActualRoot
    )

    $expectedFiles = @(Get-ChildItem -LiteralPath $ExpectedRoot -Recurse -File | Sort-Object FullName)
    $actualFiles = @(Get-ChildItem -LiteralPath $ActualRoot -Recurse -File | Sort-Object FullName)
    $expectedRelative = @($expectedFiles | ForEach-Object { [IO.Path]::GetRelativePath($ExpectedRoot, $_.FullName) })
    $actualRelative = @($actualFiles | ForEach-Object { [IO.Path]::GetRelativePath($ActualRoot, $_.FullName) })
    if (($expectedRelative -join "`n") -cne ($actualRelative -join "`n")) {
        throw "Extracted file set differs. Expected: $($expectedRelative -join ', '); actual: $($actualRelative -join ', ')"
    }
    foreach ($relative in $expectedRelative) {
        $expectedHash = (Get-FileHash -LiteralPath (Join-Path $ExpectedRoot $relative) -Algorithm SHA256).Hash
        $actualHash = (Get-FileHash -LiteralPath (Join-Path $ActualRoot $relative) -Algorithm SHA256).Hash
        if ($expectedHash -ne $actualHash) {
            throw "Extracted content differs for $relative."
        }
    }
    return $expectedRelative.Count
}

function Get-SevenZipMethods {
    param(
        [Parameter(Mandatory)][string]$Archive,
        [string]$Password
    )

    $arguments = @('l', '-slt', $Archive)
    if (-not [string]::IsNullOrEmpty($Password)) {
        $arguments += "-p$Password"
    }
    $lines = @(Invoke-SevenZip -Arguments $arguments)
    return @($lines |
        Where-Object { $_ -match '^Method = (.+)$' } |
        ForEach-Object { $Matches[1] } |
        Sort-Object -Unique)
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

    $fixture = Join-Path $testRoot 'input'
    New-Item -ItemType Directory -Path (Join-Path $fixture 'nested') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $fixture 'alpha.txt') -Value ('ZiFile ZIP method corpus ' * 512) -NoNewline
    Set-Content -LiteralPath (Join-Path $fixture 'nested\unicode-测试.txt') -Value '第三方 ZIP 方法互操作' -NoNewline
    $binary = [byte[]]::new(65536)
    for ($offset = 0; $offset -lt $binary.Length; $offset++) {
        $binary[$offset] = [byte](($offset * 31 + 17) % 256)
    }
    [IO.File]::WriteAllBytes((Join-Path $fixture 'nested\deterministic.bin'), $binary)

    $password = 'zifile-reference-corpus-password'
    $cases = @(
        [ordered]@{ name = 'store'; method = 'Copy'; expected = @('Store'); encryption = 'none' },
        [ordered]@{ name = 'deflate'; method = 'Deflate'; expected = @('Deflate'); encryption = 'none' },
        [ordered]@{ name = 'deflate64'; method = 'Deflate64'; expected = @('Deflate64'); encryption = 'none' },
        [ordered]@{ name = 'bzip2'; method = 'BZip2'; expected = @('BZip2'); encryption = 'none' },
        [ordered]@{ name = 'lzma'; method = 'LZMA'; expected = @('LZMA'); encryption = 'none' },
        [ordered]@{ name = 'xz'; method = 'XZ'; expected = @('xz'); encryption = 'none' },
        [ordered]@{ name = 'ppmd'; method = 'PPMd'; expected = @('PPMd'); encryption = 'none' },
        [ordered]@{ name = 'deflate-aes256'; method = 'Deflate'; expected = @('Deflate', 'AES-256'); encryption = 'AES256' },
        [ordered]@{ name = 'deflate-zipcrypto'; method = 'Deflate'; expected = @('Deflate', 'ZipCrypto'); encryption = 'ZipCrypto' }
    )
    $results = @()
    foreach ($case in $cases) {
        $archive = Join-Path $testRoot ("reference-$($case.name).zip")
        $arguments = @('a', '-tzip', $archive, '.', '-y', "-mm=$($case.method)")
        $encrypted = $case.encryption -ne 'none'
        if ($encrypted) {
            $arguments += "-mem=$($case.encryption)"
            $arguments += "-p$password"
        }
        Push-Location $fixture
        try {
            Invoke-SevenZip -Arguments $arguments | Out-Null
        }
        finally {
            Pop-Location
        }

        $reportedMethods = @(Get-SevenZipMethods -Archive $archive -Password $(if ($encrypted) { $password } else { $null }))
        $reportedText = $reportedMethods -join ' '
        foreach ($expected in $case.expected) {
            if ($reportedText -notmatch [regex]::Escape($expected)) {
                throw "7-Zip did not report expected method '$expected' for case $($case.name): $reportedText"
            }
        }

        if ($encrypted) {
            $password | & $cli test $archive --password-stdin | Out-Null
        }
        else {
            & $cli test $archive | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not test ZIP case $($case.name)." }
        $output = Join-Path $testRoot ("output-$($case.name)")
        if ($encrypted) {
            $password | & $cli extract $archive $output --password-stdin | Out-Null
        }
        else {
            & $cli extract $archive $output | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not extract ZIP case $($case.name)." }
        $count = Assert-FixtureMatches -ExpectedRoot $fixture -ActualRoot $output
        $results += [ordered]@{
            direction = '7zip-to-zifile'
            name = $case.name
            requested_method = $case.method
            encryption = $case.encryption
            reported_methods = $reportedMethods
            files = $count
            archive_bytes = (Get-Item -LiteralPath $archive).Length
            archive_sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
            passed = $true
        }
    }

    foreach ($encrypted in @($false, $true)) {
        $name = if ($encrypted) { 'zifile-aes256' } else { 'zifile-default' }
        $archive = Join-Path $testRoot "$name.zip"
        if ($encrypted) {
            $password | & $cli create $archive $fixture --format zip --password-stdin | Out-Null
        }
        else {
            & $cli create $archive $fixture --format zip | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not create $name.zip." }
        $testArguments = @('t', $archive, '-y')
        if ($encrypted) { $testArguments += "-p$password" }
        Invoke-SevenZip -Arguments $testArguments | Out-Null
        $output = Join-Path $testRoot "$name-output"
        $extractArguments = @('x', $archive, "-o$output", '-y')
        if ($encrypted) { $extractArguments += "-p$password" }
        Invoke-SevenZip -Arguments $extractArguments | Out-Null
        $count = Assert-FixtureMatches -ExpectedRoot $fixture -ActualRoot (Join-Path $output 'input')
        $reportedMethods = @(Get-SevenZipMethods -Archive $archive -Password $(if ($encrypted) { $password } else { $null }))
        $results += [ordered]@{
            direction = 'zifile-to-7zip'
            name = $name
            requested_method = 'Deflate'
            encryption = $(if ($encrypted) { 'AES256' } else { 'none' })
            reported_methods = $reportedMethods
            files = $count
            archive_bytes = (Get-Item -LiteralPath $archive).Length
            archive_sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
            passed = $true
        }
    }

    $evidence = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTime]::UtcNow.ToString('o')
        seven_zip_version = $sevenZipVersion
        reference_cases = $cases.Count
        reverse_cases = 2
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
            Remove-Item -LiteralPath $resolved -Recurse -Force
        }
    }
}
