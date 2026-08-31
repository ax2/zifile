param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$testRoot = [IO.Path]::GetFullPath((Join-Path $tempRoot ("zifile-contract-" + [guid]::NewGuid().ToString('N'))))
if (-not $testRoot.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create contract fixtures outside the temporary directory.'
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

    $rootHelp = (& $cli --help 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'The CLI top-level help returned a non-zero exit code.' }
    foreach ($command in @('formats', 'detect', 'list', 'test', 'extract', 'create', 'update')) {
        if ($rootHelp -notmatch ("(?m)^  " + [Regex]::Escape($command) + '\s')) {
            throw "The CLI top-level help omits the public command: $command"
        }
    }

    $createHelp = (& $cli create --help 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0 -or
        $createHelp -notmatch '--password-stdin' -or
        $createHelp -match '(?m)^\s*--password(?:\s|=|<)') {
        throw 'The CLI create help violates the password-input contract.'
    }

    $formatValues = @(
        'zip', 'seven-zip', 'tar', 'tar-gzip', 'tar-zstd', 'tar-xz', 'tar-lzma',
        'tar-bzip2', 'gzip', 'zstandard', 'xz', 'lzma', 'bzip2', 'lz4', 'brotli'
    )
    $formatValueLine = '[possible values: {0}]' -f ($formatValues -join ', ')
    if ($createHelp -notmatch [Regex]::Escape($formatValueLine)) {
        throw 'The CLI create help format enum drifted from the public contract.'
    }

    $updateHelp = (& $cli update --help 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0 -or
        $updateHelp -notmatch '--password-stdin' -or
        $updateHelp -match '(?m)^\s*--password(?:\s|=|<)') {
        throw 'The CLI update help violates the password-input contract.'
    }

    $formats = (& $cli formats 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'The CLI format capability command failed.' }
    $formatLines = @($formats -split "`r?`n" | Where-Object { $_.Length -gt 0 })
    $expectedRows = @(
        "FORMAT`tLIST`tEXTRACT`tCREATE`tCREATE_INPUT`tCOMPRESSION_LEVEL`tENCRYPTION`tSTAGE",
        "ZIP`tyes`tyes`tyes`tfiles-or-directories`t0-9`tyes`tAlpha",
        "7z`tyes`tyes`tyes`tfiles-or-directories`t0-9`tyes`tAlpha",
        "TAR`tyes`tyes`tyes`tfiles-or-directories`tfixed`tno`tAlpha",
        "TAR + gzip`tyes`tyes`tyes`tfiles-or-directories`t0-9`tno`tAlpha",
        "TAR + Zstandard`tyes`tyes`tyes`tfiles-or-directories`t0-22`tno`tAlpha",
        "TAR + XZ`tyes`tyes`tyes`tfiles-or-directories`t0-9`tno`tAlpha",
        "TAR + LZMA`tyes`tyes`tyes`tfiles-or-directories`t0-9`tno`tAlpha",
        "TAR + Bzip2`tyes`tyes`tyes`tfiles-or-directories`t1-9`tno`tAlpha",
        "gzip`tyes`tyes`tyes`tsingle-file`t0-9`tno`tAlpha",
        "Zstandard`tyes`tyes`tyes`tsingle-file`t0-22`tno`tAlpha",
        "XZ`tyes`tyes`tyes`tsingle-file`t0-9`tno`tAlpha",
        "LZMA`tyes`tyes`tyes`tsingle-file`t0-9`tno`tAlpha",
        "Bzip2`tyes`tyes`tyes`tsingle-file`t1-9`tno`tAlpha",
        "LZ4`tyes`tyes`tyes`tsingle-file`tfixed`tno`tAlpha",
        "Brotli`tyes`tyes`tyes`tsingle-file`t0-11`tno`tAlpha",
        "RAR`tyes`tyes`tno`tnone`tnone`tyes`tBeta",
        "CAB`tyes`tyes`tno`tnone`tnone`tno`tBeta"
    )
    if ($formatLines.Count -ne $expectedRows.Count) {
        throw "The CLI format matrix row count changed from $($expectedRows.Count) to $($formatLines.Count)."
    }
    for ($index = 0; $index -lt $expectedRows.Count; $index++) {
        if ($formatLines[$index] -cne $expectedRows[$index]) {
            throw "The CLI format matrix row $index drifted: '$($formatLines[$index])'."
        }
    }

    $docs = @(
        (Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'docs\src\content\docs\development\contracts.md')),
        (Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'docs\src\content\docs\en\development\contracts.md'))
    )
    foreach ($contract in $docs) {
        foreach ($value in $formatValues) {
            if ($contract -notmatch ('`' + [Regex]::Escape($value) + '`')) {
                throw "The public contract documentation omits create format '$value'."
            }
        }
        foreach ($value in @('overwrite', 'skip', 'rename', 'error')) {
            if ($contract -notmatch ('`' + [Regex]::Escape($value) + '`')) {
                throw "The public contract documentation omits conflict value '$value'."
            }
        }
    }

    New-Item -ItemType Directory -Path (Join-Path $testRoot 'input') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $testRoot 'input\hello.txt') -Value 'contract smoke' -NoNewline

    $missingPath = Join-Path $testRoot 'missing.zip'
    $detectError = (& $cli detect $missingPath 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 1 -or $detectError -notmatch '(?m)^error: ') {
        throw "Runtime error contract failed: exit=$LASTEXITCODE output=$detectError"
    }

    $invalidDestination = Join-Path $testRoot 'invalid.zip'
    $invalidError = (& $cli create $invalidDestination (Join-Path $testRoot 'input\hello.txt') `
        --format not-a-format 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 2 -or $invalidError -notmatch '(?m)(invalid value|possible values)') {
        throw "CLI syntax error contract failed: exit=$LASTEXITCODE output=$invalidError"
    }
    if (Test-Path -LiteralPath $invalidDestination) {
        throw 'CLI syntax rejection created an output file.'
    }

    $archive = Join-Path $testRoot 'update.zip'
    $addition = Join-Path $testRoot 'addition.txt'
    Set-Content -LiteralPath $addition -Value 'updated contract smoke' -NoNewline
    $createOutput = (& $cli create $archive (Join-Path $testRoot 'input\hello.txt') --format zip --level 6 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "CLI create setup failed: $createOutput" }
    $updateOutput = (& $cli update $archive $addition --level 6 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "CLI update round trip failed: $updateOutput" }
    $extractRoot = Join-Path $testRoot 'updated-output'
    $extractOutput = (& $cli extract $archive $extractRoot --conflict error 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "CLI update extraction failed: $extractOutput" }
    $updatedFile = Join-Path $extractRoot 'addition.txt'
    if ((Get-Content -Raw -LiteralPath $updatedFile) -ne 'updated contract smoke') {
        throw 'CLI update round trip produced unexpected file content.'
    }

    $global:LASTEXITCODE = 0
    [ordered]@{
        schema_version = 1
        commands_checked = 7
        create_formats_checked = $formatValues.Count
        capability_rows_checked = $expectedRows.Count - 1
        runtime_error_exit_code = 1
        syntax_error_exit_code = 2
        bilingual_contract_docs_checked = 2
    } | ConvertTo-Json -Depth 3
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
