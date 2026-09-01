param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$testRoot = [IO.Path]::GetFullPath(
    (Join-Path $tempRoot ("zifile-cab-interop-" + [guid]::NewGuid().ToString('N')))
)
if (-not $testRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create CAB fixtures outside the system temporary directory.'
}

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        cargo build -p zifile-cli --locked
        if ($LASTEXITCODE -ne 0) { throw 'ZiFile CLI build failed.' }
    }
    $cli = Join-Path $repoRoot 'target\debug\zifile.exe'
    if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) {
        throw 'ZiFile CLI executable was not found.'
    }
    foreach ($tool in @('makecab.exe', 'expand.exe')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "Required Windows reference tool is unavailable: $tool"
        }
    }

    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    $source = Join-Path $testRoot 'cab-payload.txt'
    [IO.File]::WriteAllText(
        $source,
        ('ZiFile Windows Cabinet interoperability' + [Environment]::NewLine),
        [Text.UTF8Encoding]::new($false)
    )
    $sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
    $cases = @(
        [ordered]@{ name = 'mszip'; type = 'MSZIP'; memory = $null },
        [ordered]@{ name = 'lzx'; type = 'LZX'; memory = 21 }
    )
    $results = @()
    foreach ($case in $cases) {
        $cabinet = Join-Path $testRoot ($case.name + '.cab')
        $arguments = @('/D', "CompressionType=$($case.type)")
        if ($null -ne $case.memory) {
            $arguments += @('/D', "CompressionMemory=$($case.memory)")
        }
        $arguments += @($source, $cabinet)
        & makecab.exe @arguments | Out-Host
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $cabinet -PathType Leaf)) {
            throw "makecab failed to create the $($case.name) fixture."
        }

        $detected = (& $cli detect $cabinet) -join [Environment]::NewLine
        if ($LASTEXITCODE -ne 0 -or $detected -notmatch 'CAB') {
            throw "ZiFile did not detect the $($case.name) fixture as CAB."
        }
        $listed = (& $cli list $cabinet) -join [Environment]::NewLine
        if ($LASTEXITCODE -ne 0 -or $listed -notmatch 'cab-payload\.txt') {
            throw "ZiFile did not list the $($case.name) CAB payload."
        }
        & $cli test $cabinet
        if ($LASTEXITCODE -ne 0) {
            throw "ZiFile did not verify the $($case.name) CAB fixture."
        }

        $zifileOutput = Join-Path $testRoot ($case.name + '-zifile')
        & $cli extract $cabinet $zifileOutput
        if ($LASTEXITCODE -ne 0) {
            throw "ZiFile did not extract the $($case.name) CAB fixture."
        }
        $zifileHash = (
            Get-FileHash -LiteralPath (Join-Path $zifileOutput 'cab-payload.txt') -Algorithm SHA256
        ).Hash
        if ($zifileHash -cne $sourceHash) {
            throw "ZiFile content differs for the $($case.name) CAB fixture."
        }

        $referenceOutput = Join-Path $testRoot ($case.name + '-expand')
        New-Item -ItemType Directory -Path $referenceOutput -Force | Out-Null
        & expand.exe $cabinet '-F:*' $referenceOutput | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "expand.exe did not extract the $($case.name) CAB fixture."
        }
        $referenceFiles = @(Get-ChildItem -LiteralPath $referenceOutput -File)
        if ($referenceFiles.Count -ne 1) {
            throw "expand.exe produced an unexpected file set for the $($case.name) CAB fixture."
        }
        $referenceHash = (Get-FileHash -LiteralPath $referenceFiles[0].FullName -Algorithm SHA256).Hash
        if ($referenceHash -cne $sourceHash) {
            throw "expand.exe content differs for the $($case.name) CAB fixture."
        }

        $results += [ordered]@{
            name = $case.name
            compression_type = $case.type
            cabinet_sha256 = (Get-FileHash -LiteralPath $cabinet -Algorithm SHA256).Hash
            source_sha256 = $sourceHash
            zifile_sha256 = $zifileHash
            expand_sha256 = $referenceHash
            matched = $true
        }
    }

    # ZiFile MSZIP -> Windows expand.exe. This is the reverse direction of the
    # makecab fixtures above and proves the generated cabinet is system-readable.
    $zifileCabinet = Join-Path $testRoot 'zifile-mszip.cab'
    & $cli create $zifileCabinet $source --format cab | Out-Host
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $zifileCabinet -PathType Leaf)) {
        throw 'ZiFile failed to create the CAB interoperability fixture.'
    }
    $zifileReferenceOutput = Join-Path $testRoot 'zifile-expand'
    New-Item -ItemType Directory -Path $zifileReferenceOutput -Force | Out-Null
    & expand.exe $zifileCabinet '-F:*' $zifileReferenceOutput | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw 'expand.exe did not extract the ZiFile-created CAB fixture.'
    }
    $zifileReferenceFiles = @(Get-ChildItem -LiteralPath $zifileReferenceOutput -File)
    if ($zifileReferenceFiles.Count -ne 1) {
        throw 'expand.exe produced an unexpected file set for the ZiFile-created CAB.'
    }
    $zifileReferenceHash = (Get-FileHash -LiteralPath $zifileReferenceFiles[0].FullName -Algorithm SHA256).Hash
    if ($zifileReferenceHash -cne $sourceHash) {
        throw 'expand.exe content differs for the ZiFile-created CAB.'
    }
    $results += [ordered]@{
        name = 'zifile-mszip'
        compression_type = 'MSZIP'
        cabinet_sha256 = (Get-FileHash -LiteralPath $zifileCabinet -Algorithm SHA256).Hash
        source_sha256 = $sourceHash
        zifile_sha256 = $sourceHash
        expand_sha256 = $zifileReferenceHash
        matched = $true
    }

    $evidence = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTimeOffset]::UtcNow.ToString('O')
        platform = 'windows'
        creator = 'makecab.exe and ZiFile CLI'
        reference_extractor = 'expand.exe'
        cases = $results
        passed = ($results.Count -eq 3 -and @($results | Where-Object { -not $_.matched }).Count -eq 0)
    }
    $evidencePath = Join-Path $repoRoot 'target\cab-interoperability.json'
    $evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $evidencePath -Encoding utf8
    if (-not $evidence.passed) {
        throw 'CAB interoperability evidence is incomplete.'
    }
    $evidence | ConvertTo-Json -Depth 6
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
