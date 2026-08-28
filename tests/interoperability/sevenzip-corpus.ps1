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
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-7zip-corpus-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the 7-Zip corpus outside the system temporary directory.'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot 'target\sevenzip-corpus.json'
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

    & $script:sevenZip @Arguments | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "7-Zip failed with exit code $LASTEXITCODE."
    }
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
    $versionMatch = [regex]::Match($versionText, '7-Zip\s+([0-9]+(?:\.[0-9]+)+)')
    $sevenZipVersion = if ($versionMatch.Success) { $versionMatch.Groups[1].Value } else { 'unknown' }

    $fixture = Join-Path $testRoot 'input'
    New-Item -ItemType Directory -Path (Join-Path $fixture 'nested') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $fixture 'alpha.txt') -Value ('ZiFile codec corpus ' * 512) -NoNewline
    Set-Content -LiteralPath (Join-Path $fixture 'nested\unicode-测试.txt') -Value '第三方 7-Zip 互操作' -NoNewline
    $codeBytes = [byte[]]::new(32768)
    for ($offset = 0; $offset -lt $codeBytes.Length; $offset += 8) {
        $codeBytes[$offset] = 0xE8
        [BitConverter]::GetBytes([int]($offset * 3)).CopyTo($codeBytes, $offset + 1)
        $codeBytes[$offset + 5] = 0x90
        $codeBytes[$offset + 6] = 0x90
        $codeBytes[$offset + 7] = 0xC3
    }
    [IO.File]::WriteAllBytes((Join-Path $fixture 'nested\x86-pattern.bin'), $codeBytes)

    $password = 'zifile-reference-corpus-password'
    $cases = @(
        [ordered]@{ name = 'copy-nonsolid'; arguments = @('-m0=Copy', '-ms=off'); encrypted = $false },
        [ordered]@{ name = 'lzma-solid'; arguments = @('-m0=LZMA', '-ms=on'); encrypted = $false },
        [ordered]@{ name = 'lzma2-bcj'; arguments = @('-m0=BCJ', '-m1=LZMA2', '-ms=on'); encrypted = $false },
        [ordered]@{ name = 'deflate'; arguments = @('-m0=Deflate', '-ms=off'); encrypted = $false },
        [ordered]@{ name = 'bzip2'; arguments = @('-m0=BZip2', '-ms=off'); encrypted = $false },
        [ordered]@{ name = 'ppmd'; arguments = @('-m0=PPMd', '-ms=off'); encrypted = $false },
        [ordered]@{ name = 'lzma2-aes-headers'; arguments = @('-m0=LZMA2', '-mhe=on'); encrypted = $true }
    )
    $results = @()
    foreach ($case in $cases) {
        $archive = Join-Path $testRoot ("reference-$($case.name).7z")
        $arguments = @('a', '-t7z', $archive, '.', '-y') + @($case.arguments)
        if ($case.encrypted) {
            $arguments += "-p$password"
        }
        Push-Location $fixture
        try {
            Invoke-SevenZip -Arguments $arguments
        } finally {
            Pop-Location
        }

        if ($case.encrypted) {
            $password | & $cli test $archive --password-stdin | Out-Null
        } else {
            & $cli test $archive | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not test 7-Zip case $($case.name)." }
        $output = Join-Path $testRoot ("output-$($case.name)")
        if ($case.encrypted) {
            $password | & $cli extract $archive $output --password-stdin | Out-Null
        } else {
            & $cli extract $archive $output | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not extract 7-Zip case $($case.name)." }
        $count = Assert-FixtureMatches -ExpectedRoot $fixture -ActualRoot $output
        $results += [ordered]@{
            name = $case.name
            encrypted = $case.encrypted
            files = $count
            archive_bytes = (Get-Item -LiteralPath $archive).Length
            archive_sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
        }
    }

    foreach ($encrypted in @($false, $true)) {
        $name = if ($encrypted) { 'zifile-aes' } else { 'zifile-default' }
        $archive = Join-Path $testRoot "$name.7z"
        if ($encrypted) {
            $password | & $cli create $archive $fixture --format seven-zip --password-stdin | Out-Null
        } else {
            & $cli create $archive $fixture --format seven-zip | Out-Null
        }
        if ($LASTEXITCODE -ne 0) { throw "ZiFile could not create $name.7z." }
        $testArguments = @('t', $archive, '-y')
        if ($encrypted) { $testArguments += "-p$password" }
        Invoke-SevenZip -Arguments $testArguments
        $output = Join-Path $testRoot "$name-output"
        $extractArguments = @('x', $archive, "-o$output", '-y')
        if ($encrypted) { $extractArguments += "-p$password" }
        Invoke-SevenZip -Arguments $extractArguments
        $count = Assert-FixtureMatches -ExpectedRoot $fixture -ActualRoot (Join-Path $output 'input')
        $results += [ordered]@{
            name = $name
            encrypted = $encrypted
            files = $count
            archive_bytes = (Get-Item -LiteralPath $archive).Length
            archive_sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
        }
    }

    $evidence = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTime]::UtcNow.ToString('o')
        seven_zip_path = $script:sevenZip
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
