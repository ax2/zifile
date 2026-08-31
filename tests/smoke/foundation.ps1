param(
    [switch]$SkipDesktopLaunch
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Push-Location $repoRoot

try {
    cargo build --workspace --locked
    if ($LASTEXITCODE -ne 0) {
        throw 'Workspace build failed.'
    }

    $cliPath = Join-Path $repoRoot 'target\debug\zifile.exe'
    if (-not (Test-Path -LiteralPath $cliPath)) {
        throw 'CLI executable was not produced.'
    }
    $formats = (& $cliPath formats) -join "`n"
    if (
        $LASTEXITCODE -ne 0 -or
        $formats -notmatch '(?m)^FORMAT\tLIST\tEXTRACT\tCREATE\tCREATE_INPUT\tCOMPRESSION_LEVEL\tENCRYPTION\tSTAGE$' -or
        $formats -notmatch '(?m)^ZIP\tyes\tyes\tyes\tfiles-or-directories\t0-9\tyes\tAlpha$' -or
        $formats -notmatch '(?m)^Zstandard\tyes\tyes\tyes\tsingle-file\t0-22\tno\tAlpha$' -or
        $formats -notmatch '(?m)^LZMA\tyes\tyes\tyes\tsingle-file\t0-9\tno\tAlpha$' -or
        $formats -notmatch '(?m)^Bzip2\tyes\tyes\tyes\tsingle-file\t1-9\tno\tAlpha$' -or
        $formats -notmatch '(?m)^TAR\tyes\tyes\tyes\tfiles-or-directories\tfixed\tno\tAlpha$' -or
        $formats -notmatch '(?m)^TAR \+ LZMA\tyes\tyes\tyes\tfiles-or-directories\t0-9\tno\tAlpha$' -or
        $formats -notmatch '(?m)^LZ4\tyes\tyes\tyes\tsingle-file\tfixed\tno\tAlpha$' -or
        $formats -notmatch '(?m)^RAR\tyes\tyes\tno\tnone\tnone\tyes\tBeta$' -or
        $formats -notmatch '(?m)^CAB\tyes\tyes\tno\tnone\tnone\tno\tBeta$'
    ) {
        throw 'CLI format registry smoke test failed.'
    }
    $createHelp = (& $cliPath create --help) -join "`n"
    if (
        $LASTEXITCODE -ne 0 -or
        $createHelp -notmatch '--password-stdin' -or
        $createHelp -match '(?m)^\s*--password(?:\s|=|<)'
    ) {
        throw 'CLI password input policy smoke test failed.'
    }

    $tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    $smokeRoot = Join-Path $tempRoot ("zifile-smoke-" + [guid]::NewGuid().ToString('N'))
    $smokeRoot = [System.IO.Path]::GetFullPath($smokeRoot)
    if (-not $smokeRoot.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to create a smoke directory outside the system temporary directory.'
    }
    New-Item -ItemType Directory -Path (Join-Path $smokeRoot 'input\nested') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $smokeRoot 'input\hello.txt') -Value 'hello ZiFile' -NoNewline
    Set-Content -LiteralPath (Join-Path $smokeRoot 'input\nested\unicode-测试.txt') -Value 'archive smoke' -NoNewline

    $invalidLevelArchive = Join-Path $smokeRoot 'invalid-level.zip'
    $invalidLevelError = (& $cliPath create $invalidLevelArchive `
        (Join-Path $smokeRoot 'input\hello.txt') --format zip --level 10 2>&1) -join "`n"
    if (
        $LASTEXITCODE -ne 1 -or
        $invalidLevelError -notmatch 'compression level for ZIP must be between 0 and 9; received 10' -or
        (Test-Path -LiteralPath $invalidLevelArchive)
    ) {
        throw "CLI format-specific compression-level rejection failed: $invalidLevelError"
    }

    $fixedLevelArchive = Join-Path $smokeRoot 'fixed-level.tar'
    $fixedLevelError = (& $cliPath create $fixedLevelArchive `
        (Join-Path $smokeRoot 'input\hello.txt') --format tar --level 6 2>&1) -join "`n"
    if (
        $LASTEXITCODE -ne 1 -or
        $fixedLevelError -notmatch 'compression level is fixed for TAR; omit --level instead of passing 6' -or
        (Test-Path -LiteralPath $fixedLevelArchive)
    ) {
        throw "CLI fixed compression-level rejection failed: $fixedLevelError"
    }

    $defaultLevelArchive = Join-Path $smokeRoot 'default-level.tar'
    & $cliPath create $defaultLevelArchive (Join-Path $smokeRoot 'input\hello.txt') --format tar
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $defaultLevelArchive)) {
        throw 'CLI fixed-format creation without --level failed.'
    }

    function Invoke-CliRoundTrip {
        param(
            [Parameter(Mandatory)][string]$Format,
            [Parameter(Mandatory)][string]$ArchiveName,
            [Parameter(Mandatory)][string]$SourcePath,
            [Parameter(Mandatory)][string]$OutputDirectory,
            [Parameter(Mandatory)][string]$ExpectedRelativePath,
            [Parameter(Mandatory)][string]$ExpectedContent
        )

        $archive = Join-Path $smokeRoot $ArchiveName
        & $cliPath create $archive $SourcePath --format $Format
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $archive -PathType Leaf)) {
            throw "CLI $Format creation smoke test failed."
        }
        & $cliPath test $archive
        if ($LASTEXITCODE -ne 0) {
            throw "CLI $Format integrity smoke test failed."
        }
        $output = Join-Path $smokeRoot $OutputDirectory
        & $cliPath extract $archive $output
        $expectedPath = Join-Path $output $ExpectedRelativePath
        if ($LASTEXITCODE -ne 0 -or
            -not (Test-Path -LiteralPath $expectedPath -PathType Leaf) -or
            (Get-Content -Raw -LiteralPath $expectedPath) -ne $ExpectedContent) {
            throw "CLI $Format extraction smoke test failed."
        }
    }

    $nestedInput = Join-Path $smokeRoot 'input'
    foreach ($case in @(
        @{ Format = 'tar-zstd'; ArchiveName = 'smoke.tar.zst'; Output = 'tar-zstd-output' },
        @{ Format = 'tar-xz'; ArchiveName = 'smoke.tar.xz'; Output = 'tar-xz-output' },
        @{ Format = 'tar-bzip2'; ArchiveName = 'smoke.tar.bz2'; Output = 'tar-bzip2-output' }
    )) {
        Invoke-CliRoundTrip `
            -Format $case.Format `
            -ArchiveName $case.ArchiveName `
            -SourcePath $nestedInput `
            -OutputDirectory $case.Output `
            -ExpectedRelativePath 'input\nested\unicode-测试.txt' `
            -ExpectedContent 'archive smoke'
    }

    foreach ($case in @(
        @{ Format = 'gzip'; ArchiveName = 'hello.txt.gz'; Output = 'gzip-output' },
        @{ Format = 'zstandard'; ArchiveName = 'hello.txt.zst'; Output = 'zstandard-output' },
        @{ Format = 'xz'; ArchiveName = 'hello.txt.xz'; Output = 'xz-output' },
        @{ Format = 'lzma'; ArchiveName = 'hello.txt.lzma'; Output = 'lzma-output' },
        @{ Format = 'bzip2'; ArchiveName = 'hello.txt.bz2'; Output = 'bzip2-output' },
        @{ Format = 'lz4'; ArchiveName = 'hello.txt.lz4'; Output = 'lz4-output' },
        @{ Format = 'brotli'; ArchiveName = 'hello.txt.br'; Output = 'brotli-output' }
    )) {
        Invoke-CliRoundTrip `
            -Format $case.Format `
            -ArchiveName $case.ArchiveName `
            -SourcePath (Join-Path $nestedInput 'hello.txt') `
            -OutputDirectory $case.Output `
            -ExpectedRelativePath 'hello.txt' `
            -ExpectedContent 'hello ZiFile'
    }

    $archivePath = Join-Path $smokeRoot 'smoke.tar.gz'
    cargo run --quiet -p zifile-cli -- create $archivePath (Join-Path $smokeRoot 'input') --format tar-gzip
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $archivePath)) {
        throw 'CLI archive creation smoke test failed.'
    }

    $lzmaArchivePath = Join-Path $smokeRoot 'smoke.tar.lzma'
    & $cliPath create $lzmaArchivePath (Join-Path $smokeRoot 'input') --format tar-lzma
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $lzmaArchivePath)) {
        throw 'CLI TAR + LZMA creation smoke test failed.'
    }
    & $cliPath test $lzmaArchivePath
    if ($LASTEXITCODE -ne 0) {
        throw 'CLI TAR + LZMA integrity smoke test failed.'
    }
    $lzmaOutput = Join-Path $smokeRoot 'tar-lzma-output'
    & $cliPath extract $lzmaArchivePath $lzmaOutput
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $lzmaOutput 'input\nested\unicode-测试.txt')) -ne 'archive smoke') {
        throw 'CLI TAR + LZMA extraction smoke test failed.'
    }

    $encryptedArchive = Join-Path $smokeRoot 'smoke-encrypted.7z'
    'zifile-smoke-password' |
        cargo run --quiet -p zifile-cli -- create $encryptedArchive `
            (Join-Path $smokeRoot 'input\hello.txt') --format seven-zip --password-stdin
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $encryptedArchive)) {
        throw 'CLI encrypted archive creation through standard input failed.'
    }
    'zifile-smoke-password' |
        cargo run --quiet -p zifile-cli -- test $encryptedArchive --password-stdin
    if ($LASTEXITCODE -ne 0) {
        throw 'CLI encrypted archive verification through standard input failed.'
    }
    $encryptedOutput = Join-Path $smokeRoot 'encrypted-output'
    'zifile-smoke-password' |
        cargo run --quiet -p zifile-cli -- extract $encryptedArchive $encryptedOutput --password-stdin
    if (
        $LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $encryptedOutput 'hello.txt')) -ne 'hello ZiFile'
    ) {
        throw 'CLI encrypted archive extraction through standard input failed.'
    }

    $detection = (cargo run --quiet -p zifile-cli -- detect $archivePath) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $detection -notmatch 'TAR \+ gzip') {
        throw 'CLI signature detection smoke test failed.'
    }
    $lzmaDetection = (cargo run --quiet -p zifile-cli -- detect $lzmaArchivePath) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $lzmaDetection -notmatch 'TAR \+ LZMA') {
        throw 'CLI TAR + LZMA format detection smoke test failed.'
    }

    $listing = (cargo run --quiet -p zifile-cli -- list $archivePath) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $listing -notmatch 'hello.txt' -or $listing -notmatch 'unicode-测试.txt') {
        throw 'CLI archive listing smoke test failed.'
    }

    cargo run --quiet -p zifile-cli -- test $archivePath
    if ($LASTEXITCODE -ne 0) {
        throw 'CLI archive integrity smoke test failed.'
    }

    $workerPath = Join-Path $repoRoot 'target\debug\zifile-worker.exe'
    if (-not (Test-Path -LiteralPath $workerPath)) {
        throw 'Worker executable was not produced.'
    }
    $workerRequest = @{
        version = 3
        payload = @{
            operation = 'list'
            archive = $archivePath
            password = $null
        }
    } | ConvertTo-Json -Depth 5 -Compress
    $workerListLines = @($workerRequest | & $workerPath)
    $workerOutput = $workerListLines -join "`n"
    if ($LASTEXITCODE -ne 0 -or $workerOutput -notmatch 'archive_start' -or
        $workerOutput -notmatch 'unicode-测试.txt' -or $workerOutput -notmatch 'archive_end') {
        throw 'Isolated worker IPC smoke test failed.'
    }
    $workerListEvents = @($workerListLines | ForEach-Object { $_ | ConvertFrom-Json })
    $listProgressEvents = @($workerListEvents | Where-Object { $_.payload.event -eq 'progress' })
    $lastListProgress = $listProgressEvents | Select-Object -Last 1
    [object[]]$workerListEventNames = @($workerListEvents | ForEach-Object { $_.payload.event })
    $listArchiveStartIndex = [Array]::IndexOf($workerListEventNames, 'archive_start')
    $lastListProgressIndex = [Array]::LastIndexOf($workerListEventNames, 'progress')
    if (
        $listProgressEvents.Count -lt 1 -or
        -not $lastListProgress -or
        $lastListProgress.payload.snapshot.total_entries -lt 1 -or
        $lastListProgress.payload.snapshot.processed_entries -ne $lastListProgress.payload.snapshot.total_entries -or
        $lastListProgressIndex -ge $listArchiveStartIndex
    ) {
        throw 'Worker listing did not emit a complete final progress snapshot before archive streaming.'
    }

    $workerTestRequest = @{
        version = 3
        payload = @{
            operation = 'test'
            archive = $archivePath
            password = $null
        }
    } | ConvertTo-Json -Depth 5 -Compress
    $workerTestLines = @($workerTestRequest | & $workerPath)
    if ($LASTEXITCODE -ne 0) {
        throw 'Worker integrity test request failed.'
    }
    $workerTestEvents = @($workerTestLines | ForEach-Object { $_ | ConvertFrom-Json })
    $progressEvents = @($workerTestEvents | Where-Object { $_.payload.event -eq 'progress' })
    $lastProgress = $progressEvents | Select-Object -Last 1
    if (
        $progressEvents.Count -lt 1 -or
        -not $lastProgress -or
        $lastProgress.payload.snapshot.total_entries -lt 1 -or
        $lastProgress.payload.snapshot.processed_entries -ne $lastProgress.payload.snapshot.total_entries -or
        $lastProgress.payload.snapshot.processed_bytes -ne $lastProgress.payload.snapshot.total_bytes -or
        $workerTestEvents[-1].payload.event -ne 'archive_end'
    ) {
        throw 'Worker integrity test did not emit a complete final progress snapshot before its archive result.'
    }

    $workerRenameArchive = Join-Path $smokeRoot 'worker-rename.tar.gz'
    Copy-Item -LiteralPath $archivePath -Destination $workerRenameArchive
    $workerRenameRequest = @{
        version = 3
        payload = @{
            operation = 'rename'
            archive = $workerRenameArchive
            renames = @(@{ from = 'input/hello.txt'; to = 'input/renamed.txt' })
            compression_level = 6
            password = $null
            limits = @{
                max_entries = 1000000
                max_expanded_bytes = 549755813888
                max_expansion_ratio = 1000
                max_path_depth = 128
            }
        }
    } | ConvertTo-Json -Depth 8 -Compress
    $workerRenameLines = @($workerRenameRequest | & $workerPath)
    $workerRenameOutput = $workerRenameLines -join "`n"
    if ($LASTEXITCODE -ne 0 -or
        $workerRenameOutput -notmatch '"event":"summary"' -or
        $workerRenameOutput -match '"event":"error"') {
        throw "Worker archive rename smoke test failed: $workerRenameOutput"
    }
    $renamedListing = (& $cliPath list $workerRenameArchive 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0 -or
        $renamedListing -notmatch 'renamed.txt' -or
        $renamedListing -match 'hello.txt') {
        throw 'Worker archive rename did not persist the expected archive-relative path.'
    }

    $cancelRoot = Join-Path $smokeRoot 'worker-cancel'
    New-Item -ItemType Directory -Path $cancelRoot | Out-Null
    $cancelSource = Join-Path $cancelRoot 'random.bin'
    $cancelDestination = Join-Path $cancelRoot 'cancelled.7z'
    $sourceStream = [System.IO.File]::Create($cancelSource)
    try {
        $buffer = New-Object byte[] (1024 * 1024)
        for ($index = 0; $index -lt 32; $index++) {
            [System.Security.Cryptography.RandomNumberGenerator]::Fill($buffer)
            $sourceStream.Write($buffer, 0, $buffer.Length)
        }
    }
    finally {
        $sourceStream.Dispose()
    }
    $createRequest = @{
        version = 3
        payload = @{
            operation = 'create'
            sources = @($cancelSource)
            destination = $cancelDestination
            format = 'SevenZip'
            compression_level = 9
            password = $null
        }
    } | ConvertTo-Json -Depth 6 -Compress
    $cancelRequest = @{
        version = 3
        payload = @{ control = 'cancel' }
    } | ConvertTo-Json -Depth 4 -Compress
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $workerPath
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $worker = [System.Diagnostics.Process]::new()
    $worker.StartInfo = $startInfo
    if (-not $worker.Start()) {
        throw 'Cancellation smoke worker did not start.'
    }
    $cancelOutputTask = $worker.StandardOutput.ReadToEndAsync()
    $cancelErrorTask = $worker.StandardError.ReadToEndAsync()
    $worker.StandardInput.WriteLine($createRequest)
    $worker.StandardInput.Flush()
    Start-Sleep -Milliseconds 100
    $worker.StandardInput.WriteLine($cancelRequest)
    $worker.StandardInput.Close()
    if (-not $worker.WaitForExit(10000)) {
        $worker.Kill()
        throw 'Worker did not stop after cooperative cancellation.'
    }
    $cancelOutput = $cancelOutputTask.Result
    $cancelError = $cancelErrorTask.Result
    if ($worker.ExitCode -eq 0 -or $cancelOutput -notmatch '"event":"error"' -or
        $cancelOutput -notmatch 'Cancelled') {
        throw "Worker did not acknowledge cancellation. stdout: $cancelOutput stderr: $cancelError"
    }
    if (Test-Path -LiteralPath $cancelDestination) {
        throw 'Cancelled archive destination remained on disk.'
    }
    $cancelArtifacts = @(Get-ChildItem -LiteralPath $cancelRoot -Force |
        Where-Object Name -ne 'random.bin')
    if ($cancelArtifacts.Count -ne 0) {
        throw "Cancelled Worker left temporary artifacts: $($cancelArtifacts.Name -join ', ')"
    }

    $extractPath = Join-Path $smokeRoot 'output'
    cargo run --quiet -p zifile-cli -- extract $archivePath $extractPath
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $extractPath 'input\hello.txt')) -ne 'hello ZiFile') {
        throw 'CLI safe extraction smoke test failed.'
    }

    $desktopPath = Join-Path $repoRoot 'target\debug\zifile-desktop.exe'
    if (-not (Test-Path -LiteralPath $desktopPath)) {
        throw 'Desktop executable was not produced.'
    }

    if (-not $SkipDesktopLaunch) {
        $process = Start-Process -FilePath $desktopPath -PassThru -WindowStyle Hidden
        Start-Sleep -Seconds 3
        if ($process.HasExited) {
            throw "Desktop smoke test exited unexpectedly with code $($process.ExitCode)."
        }
        Stop-Process -Id $process.Id
    }

    Write-Host 'ZiFile archive and desktop smoke test passed.'
}
finally {
    if ($smokeRoot -and (Test-Path -LiteralPath $smokeRoot)) {
        $resolvedSmokeRoot = [System.IO.Path]::GetFullPath($smokeRoot)
        $resolvedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
        if ($resolvedSmokeRoot.StartsWith($resolvedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolvedSmokeRoot -Recurse -Force
        }
    }
    Pop-Location
}
