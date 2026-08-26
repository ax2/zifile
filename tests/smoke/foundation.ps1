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

    $formats = (cargo run --quiet -p zifile-cli -- formats) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $formats -notmatch 'ZIP' -or $formats -notmatch '7z' -or $formats -notmatch 'CAB') {
        throw 'CLI format registry smoke test failed.'
    }
    $createHelp = (cargo run --quiet -p zifile-cli -- create --help) -join "`n"
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

    $archivePath = Join-Path $smokeRoot 'smoke.tar.gz'
    cargo run --quiet -p zifile-cli -- create $archivePath (Join-Path $smokeRoot 'input') --format tar-gzip
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $archivePath)) {
        throw 'CLI archive creation smoke test failed.'
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
        version = 1
        payload = @{
            operation = 'list'
            archive = $archivePath
            password = $null
        }
    } | ConvertTo-Json -Depth 5 -Compress
    $workerOutput = ($workerRequest | & $workerPath) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $workerOutput -notmatch 'archive_start' -or
        $workerOutput -notmatch 'unicode-测试.txt' -or $workerOutput -notmatch 'archive_end') {
        throw 'Isolated worker IPC smoke test failed.'
    }

    $workerTestRequest = @{
        version = 1
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
        version = 1
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
        version = 1
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
