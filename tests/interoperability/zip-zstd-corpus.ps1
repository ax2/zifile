param(
    [switch]$SkipBuild,
    [string]$EvidencePath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd(
    [IO.Path]::DirectorySeparatorChar,
    [IO.Path]::AltDirectorySeparatorChar
)
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-zip-zstd-corpus-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the ZIP Zstandard corpus outside the system temporary directory.'
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot 'target\zip-zstd-corpus.json'
}
$EvidencePath = [IO.Path]::GetFullPath($EvidencePath)
$sourceCommit = 'ee079b86fbd3817c53fe245bea4effaaaf1d97f7'
$sourceBase = "https://raw.githubusercontent.com/libarchive/libarchive/$sourceCommit/libarchive/test"

function Get-Sha256Bytes {
    param([Parameter(Mandatory)][byte[]]$Bytes)

    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($Bytes))
}

function Get-Sha256File {
    param([Parameter(Mandatory)][string]$Path)

    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
}

function Receive-PinnedUu {
    param(
        [Parameter(Mandatory)][Net.Http.HttpClient]$Client,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$ExpectedSha256
    )

    $bytes = $Client.GetByteArrayAsync("$sourceBase/$Name").GetAwaiter().GetResult()
    if ((Get-Sha256Bytes -Bytes $bytes) -ne $ExpectedSha256) {
        throw "Pinned libarchive UU hash mismatch for $Name."
    }
    return ,$bytes
}

function ConvertFrom-UuBytes {
    param(
        [Parameter(Mandatory)][byte[]]$Bytes,
        [Parameter(Mandatory)][string]$ExpectedName,
        [int]$MaximumDecodedBytes = 16777216
    )

    $text = [Text.Encoding]::ASCII.GetString($Bytes)
    $lines = $text -split "`r?`n"
    $begin = "begin 644 $ExpectedName"
    $beginIndex = [Array]::IndexOf($lines, $begin)
    if ($beginIndex -lt 0) {
        throw "UU input does not contain the expected header: $begin"
    }
    $output = [IO.MemoryStream]::new()
    $foundEnd = $false
    try {
        for ($lineIndex = $beginIndex + 1; $lineIndex -lt $lines.Count; $lineIndex++) {
            $line = $lines[$lineIndex]
            if ($line -eq 'end') {
                $foundEnd = $true
                break
            }
            if ($line.Length -eq 0) {
                continue
            }
            $decodedCount = (([int][char]$line[0]) - 32) -band 63
            if ($decodedCount -gt 45) {
                throw "UU line declares an invalid decoded length: $decodedCount"
            }
            if ($decodedCount -eq 0) {
                continue
            }
            $encodedCount = [int]([Math]::Ceiling($decodedCount / 3.0) * 4)
            if ($line.Length -lt 1 + $encodedCount) {
                throw 'UU line is shorter than its declared payload.'
            }
            $written = 0
            for ($offset = 1; $written -lt $decodedCount; $offset += 4) {
                $values = @()
                for ($index = 0; $index -lt 4; $index++) {
                    $values += ((([int][char]$line[$offset + $index]) - 32) -band 63)
                }
                $triplet = @(
                    ((($values[0] -shl 2) -bor ($values[1] -shr 4)) -band 255),
                    ((($values[1] -shl 4) -bor ($values[2] -shr 2)) -band 255),
                    ((($values[2] -shl 6) -bor $values[3]) -band 255)
                )
                foreach ($value in $triplet) {
                    if ($written -lt $decodedCount) {
                        $output.WriteByte($value)
                        $written++
                        if ($output.Length -gt $MaximumDecodedBytes) {
                            throw 'UU input exceeds the bounded decoded-size limit.'
                        }
                    }
                }
            }
        }
        if (-not $foundEnd) {
            throw 'UU input does not contain an end marker.'
        }
        return ,$output.ToArray()
    }
    finally {
        $output.Dispose()
    }
}

function Assert-ExtractedFiles {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][object[]]$ExpectedFiles
    )

    $actualFiles = @(Get-ChildItem -LiteralPath $Root -Recurse -File | ForEach-Object {
        [pscustomobject]@{
            path = [IO.Path]::GetRelativePath($Root, $_.FullName)
            bytes = $_.Length
            sha256 = Get-Sha256File -Path $_.FullName
        }
    } | Sort-Object path)
    $expected = @($ExpectedFiles | Sort-Object path)
    if ($actualFiles.Count -ne $expected.Count) {
        throw "Expected $($expected.Count) extracted files, found $($actualFiles.Count)."
    }
    for ($index = 0; $index -lt $expected.Count; $index++) {
        if ($actualFiles[$index].path -cne $expected[$index].path -or
            $actualFiles[$index].bytes -ne $expected[$index].bytes -or
            $actualFiles[$index].sha256 -ne $expected[$index].sha256) {
            throw "Extracted ZIP Zstandard content differs for $($expected[$index].path)."
        }
    }
    return $actualFiles
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

    $vimrc = [pscustomobject]@{ path = 'vimrc'; bytes = 912; sha256 = 'B16E85E457397AB2043A7EE0A3C84307C6B4EAC157FD0B721694761F25B3ED5B' }
    $cases = @(
        [pscustomobject]@{
            name = 'single'
            uu_name = 'test_read_format_zip_zstd.zipx.uu'
            uu_sha256 = 'CD0E2B6A53461D137341F8AD306E9A62AC93CDFC45B62E2E1545CF4E9A90EE31'
            archive_name = 'test_read_format_zip_zstd.zipx'
            archive_sha256 = 'CA15C7EABDE8B918F1C91FF3A173012175B0599664F007FC3CB0BDF00C767377'
            expected_files = @($vimrc)
        },
        [pscustomobject]@{
            name = 'multi'
            uu_name = 'test_read_format_zip_zstd_multi.zipx.uu'
            uu_sha256 = '72B51B70DDD798215E527F6E7C3DE956A60733CB8D6CFC50F47755DCC19BDA68'
            archive_name = 'test_read_format_zip_zstd_multi.zipx'
            archive_sha256 = 'E01A2667AEAD4990A82A7BBB5F459DA8C32454E4D4348C06C07C8A51830F657C'
            expected_files = @(
                [pscustomobject]@{ path = 'smartd.conf'; bytes = 6699; sha256 = 'F0539B3E232908D24E41A4D852410A988ADEF8C77DD4B0215BC2A2122ABD63E1' },
                [pscustomobject]@{ path = 'ts.conf'; bytes = 852; sha256 = '7F808972C5918DA45470F50C0350808950E87123FD0804F5815DBD90EB7149B9' },
                $vimrc
            )
        }
    )

    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    $http = [Net.Http.HttpClient]::new()
    $results = @()
    try {
        foreach ($case in $cases) {
            $uuBytes = Receive-PinnedUu -Client $http -Name $case.uu_name -ExpectedSha256 $case.uu_sha256
            $archiveBytes = ConvertFrom-UuBytes -Bytes $uuBytes -ExpectedName $case.archive_name
            if ((Get-Sha256Bytes -Bytes $archiveBytes) -ne $case.archive_sha256) {
                throw "Decoded ZIP Zstandard archive hash mismatch for $($case.name)."
            }
            $archive = Join-Path $testRoot $case.archive_name
            [IO.File]::WriteAllBytes($archive, $archiveBytes)

            & $cli test $archive | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "ZiFile could not test ZIP Zstandard case $($case.name)." }
            & $cli list $archive | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "ZiFile could not list ZIP Zstandard case $($case.name)." }
            $output = Join-Path $testRoot ("output-$($case.name)")
            & $cli extract $archive $output | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "ZiFile could not extract ZIP Zstandard case $($case.name)." }
            $files = Assert-ExtractedFiles -Root $output -ExpectedFiles $case.expected_files

            $results += [ordered]@{
                name = $case.name
                source_uu_bytes = $uuBytes.Length
                source_uu_sha256 = $case.uu_sha256
                archive_bytes = $archiveBytes.Length
                archive_sha256 = $case.archive_sha256
                files = @($files)
                passed = $true
            }
        }
    }
    finally {
        $http.Dispose()
    }

    $evidence = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTime]::UtcNow.ToString('o')
        source_repository = 'libarchive/libarchive'
        source_commit = $sourceCommit
        compression_method = 'Zstandard'
        archive_cases = $results
        total_files = @($results.files).Count
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
