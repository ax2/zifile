param(
    [switch]$SkipBuild,
    [string]$EvidencePath = 'target/windows-tools-interoperability.json'
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-interop-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create interoperability fixtures outside the temporary directory.'
}
$evidenceFile = if ([IO.Path]::IsPathRooted($EvidencePath)) {
    [IO.Path]::GetFullPath($EvidencePath)
} else {
    [IO.Path]::GetFullPath((Join-Path $repoRoot $EvidencePath))
}
$interopCases = [System.Collections.Generic.List[string]]::new()
$passed = $false

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
    $interopCases.Add('powershell-zip-to-zifile')
    $interopCases.Add('zifile-zip-to-powershell')

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
    $interopCases.Add('bsdtar-tar-gzip-to-zifile')
    $interopCases.Add('zifile-tar-gzip-to-bsdtar')

    # Reference bsdtar/libarchive TAR + LZMA-alone -> ZiFile.
    $referenceTarLzma = Join-Path $testRoot 'reference.tar.lzma'
    & tar.exe -c --lzma -f $referenceTarLzma -C $fixture alpha.txt nested
    if ($LASTEXITCODE -ne 0) { throw 'Reference tar.lzma creation failed.' }
    & $cli test $referenceTarLzma
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile could not verify the reference tar.lzma.' }
    $referenceTarLzmaOutput = Join-Path $testRoot 'reference-tar-lzma-output'
    & $cli extract $referenceTarLzma $referenceTarLzmaOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $referenceTarLzmaOutput 'nested\unicode-测试.txt')) -ne '互操作') {
        throw 'ZiFile could not extract the reference tar.lzma.'
    }

    # ZiFile TAR + LZMA-alone -> reference bsdtar/libarchive.
    $zifileTarLzma = Join-Path $testRoot 'zifile.tar.lzma'
    & $cli create $zifileTarLzma $fixture --format tar-lzma
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile tar.lzma creation failed.' }
    $zifileTarLzmaOutput = Join-Path $testRoot 'zifile-tar-lzma-output'
    New-Item -ItemType Directory -Path $zifileTarLzmaOutput -Force | Out-Null
    & tar.exe -x --lzma -f $zifileTarLzma -C $zifileTarLzmaOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $zifileTarLzmaOutput 'input\alpha.txt')) -ne 'alpha from ZiFile') {
        throw 'bsdtar could not extract the ZiFile tar.lzma.'
    }
    $interopCases.Add('bsdtar-tar-lzma-to-zifile')
    $interopCases.Add('zifile-tar-lzma-to-bsdtar')

    # Windows bsdtar/libarchive 7z -> ZiFile.
    $referenceSevenZip = Join-Path $testRoot 'reference.7z'
    & tar.exe -a -cf $referenceSevenZip -C $fixture alpha.txt nested
    if ($LASTEXITCODE -ne 0) { throw 'Reference 7z creation failed.' }
    & $cli test $referenceSevenZip
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile could not verify the reference 7z.' }
    $referenceSevenZipOutput = Join-Path $testRoot 'reference-7z-output'
    & $cli extract $referenceSevenZip $referenceSevenZipOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $referenceSevenZipOutput 'nested\unicode-测试.txt')) -ne '互操作') {
        throw 'ZiFile could not extract the reference 7z.'
    }

    # ZiFile 7z -> Windows bsdtar/libarchive.
    $zifileSevenZip = Join-Path $testRoot 'zifile.7z'
    & $cli create $zifileSevenZip $fixture --format seven-zip
    if ($LASTEXITCODE -ne 0) { throw 'ZiFile 7z creation failed.' }
    $zifileSevenZipOutput = Join-Path $testRoot 'zifile-7z-output'
    New-Item -ItemType Directory -Path $zifileSevenZipOutput -Force | Out-Null
    & tar.exe -xf $zifileSevenZip -C $zifileSevenZipOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $zifileSevenZipOutput 'input\nested\unicode-测试.txt')) -ne '互操作') {
        throw 'bsdtar could not extract the ZiFile 7z.'
    }

    $interopCases.Add('bsdtar-7z-to-zifile')
    $interopCases.Add('zifile-7z-to-bsdtar')
    $passed = $true
    Write-Host 'ZIP, tar.gz, tar.lzma and 7z reference-tool interoperability passed.'
}
finally {
    $evidenceDirectory = Split-Path -Parent $evidenceFile
    if (-not (Test-Path -LiteralPath $evidenceDirectory)) {
        New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
    }
    [ordered]@{
        schema_version = 1
        passed = $passed
        reference_tool = 'Windows tar.exe (bsdtar/libarchive) plus PowerShell archive cmdlets'
        cases = @($interopCases)
        contains_user_data = $false
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $evidenceFile -Encoding utf8
    Pop-Location
    if (Test-Path -LiteralPath $testRoot) {
        $resolved = [IO.Path]::GetFullPath($testRoot)
        if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolved -Recurse -Force
        }
    }
}
