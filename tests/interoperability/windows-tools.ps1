param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-interop-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create interoperability fixtures outside the temporary directory.'
}

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        cargo build --workspace --locked
        if ($LASTEXITCODE -ne 0) { throw 'Workspace build failed.' }
    }
    $cli = Join-Path $repoRoot 'target\debug\zifile.exe'
    if (-not (Test-Path -LiteralPath $cli)) { throw 'ZiFile CLI executable was not found.' }

    $fixture = Join-Path $testRoot 'input'
    New-Item -ItemType Directory -Path (Join-Path $fixture 'nested') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $fixture 'alpha.txt') -Value 'alpha from ZiFile' -NoNewline
    Set-Content -LiteralPath (Join-Path $fixture 'nested\unicode-测试.txt') -Value '互操作' -NoNewline

    # Reference ZIP -> ZiFile.
    $referenceZip = Join-Path $testRoot 'reference.zip'
    Compress-Archive -Path (Join-Path $fixture '*') -DestinationPath $referenceZip
    & $cli test $referenceZip
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile could not verify the PowerShell ZIP.' }
    $referenceZipOutput = Join-Path $testRoot 'reference-zip-output'
    & $cli extract $referenceZip $referenceZipOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $referenceZipOutput 'alpha.txt')) -ne 'alpha from ZiFile') {
        throw 'ZiFile could not extract the PowerShell ZIP.'
    }

    # ZiFile ZIP -> reference tool.
    $zifileZip = Join-Path $testRoot 'zifile.zip'
    & $cli create $zifileZip $fixture --format zip
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile ZIP creation failed.' }
    $zifileZipOutput = Join-Path $testRoot 'zifile-zip-output'
    Expand-Archive -LiteralPath $zifileZip -DestinationPath $zifileZipOutput
    if ((Get-Content -Raw -LiteralPath (Join-Path $zifileZipOutput 'input\nested\unicode-测试.txt')) -ne '互操作') {
        throw 'PowerShell could not extract the ZiFile ZIP.'
    }

    # bsdtar -> ZiFile.
    $referenceTar = Join-Path $testRoot 'reference.tar.gz'
    & tar.exe -czf $referenceTar -C $fixture alpha.txt nested
    if ($LASTEXITCODE -ne 0) { throw 'Reference tar.gz creation failed.' }
    & $cli test $referenceTar
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile could not verify the reference tar.gz.' }
    $referenceTarOutput = Join-Path $testRoot 'reference-tar-output'
    & $cli extract $referenceTar $referenceTarOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $referenceTarOutput 'nested\unicode-测试.txt')) -ne '互操作') {
        throw 'ZiFile could not extract the reference tar.gz.'
    }

    # ZiFile -> bsdtar.
    $zifileTar = Join-Path $testRoot 'zifile.tar.gz'
    & $cli create $zifileTar $fixture --format tar-gzip
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile tar.gz creation failed.' }
    $zifileTarOutput = Join-Path $testRoot 'zifile-tar-output'
    New-Item -ItemType Directory -Path $zifileTarOutput -Force | Out-Null
    & tar.exe -xzf $zifileTar -C $zifileTarOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $zifileTarOutput 'input\alpha.txt')) -ne 'alpha from ZiFile') {
        throw 'bsdtar could not extract the ZiFile tar.gz.'
    }

    Write-Host 'ZIP and tar.gz reference-tool interoperability passed.'
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $testRoot) {
        $resolved = [IO.Path]::GetFullPath($testRoot)
        if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolved -Recurse -Force
        }
    }
}
