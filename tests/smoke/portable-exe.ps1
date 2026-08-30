param(
    [Parameter(Mandatory)][string]$ExecutablePath,
    [ValidateRange(5, 120)][int]$TimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$executable = [IO.Path]::GetFullPath($ExecutablePath)
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Portable executable does not exist: $executable"
}

$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd(
    [IO.Path]::DirectorySeparatorChar,
    [IO.Path]::AltDirectorySeparatorChar
)
$temporaryRoot = Join-Path $temporaryBase "zifile-portable-smoke-$([Guid]::NewGuid().ToString('N'))"
$portableExecutable = Join-Path $temporaryRoot 'zifile.exe'
$workerExecutable = Join-Path $temporaryRoot 'zifile-worker.exe'
$sourceRoot = Join-Path $temporaryRoot 'source'
$sourceFile = Join-Path $sourceRoot 'hello.txt'
$archivePath = Join-Path $temporaryRoot 'sample.zip'

if (-not $temporaryRoot.StartsWith($temporaryBase + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to create the portable smoke fixture outside the system temporary directory: $temporaryRoot"
}

try {
    New-Item -ItemType Directory -Path $sourceRoot -Force | Out-Null
    Set-Content -LiteralPath $sourceFile -Value 'ZiFile standalone smoke test' -Encoding utf8NoBOM
    Compress-Archive -LiteralPath $sourceFile -DestinationPath $archivePath
    Copy-Item -LiteralPath $executable -Destination $portableExecutable

    if (Test-Path -LiteralPath $workerExecutable) {
        throw 'The standalone portable smoke directory unexpectedly contains a separate Worker executable.'
    }

    $request = [ordered]@{
        version = 1
        payload = [ordered]@{
            operation = 'list'
            archive = $archivePath
            password = $null
        }
    } | ConvertTo-Json -Compress

    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $portableExecutable
    $startInfo.ArgumentList.Add('--zifile-worker')
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo

    if (-not $process.Start()) {
        throw 'The standalone portable executable did not start.'
    }
    $process.StandardInput.WriteLine($request)
    $process.StandardInput.Close()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $process.Kill($true)
        $process.WaitForExit()
        throw "The standalone portable executable did not finish within $TimeoutSeconds seconds."
    }
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    $exitCode = $process.ExitCode
    $process.Dispose()

    if ($exitCode -ne 0) {
        throw "The standalone portable executable exited with code $exitCode`: $stderr"
    }

    $events = @(
        $stdout -split '\r?\n' |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_ | ConvertFrom-Json }
    )
    $eventTypes = @($events | ForEach-Object { $_.payload.event })
    if ($eventTypes -notcontains 'archive_start' -or
        $eventTypes -notcontains 'archive_entry' -or
        $eventTypes -notcontains 'archive_end') {
        throw "The standalone portable executable did not complete a list operation. Events: $($eventTypes -join ', ')"
    }

    $entryPaths = @(
        $events |
            Where-Object { $_.payload.event -eq 'archive_entry' } |
            ForEach-Object { [string]$_.payload.entry.path }
    )
    if ($entryPaths -notcontains 'hello.txt') {
        throw "The standalone portable executable did not return the expected archive entry. Entries: $($entryPaths -join ', ')"
    }

    [ordered]@{
        schema_version = 1
        executable = [IO.Path]::GetFileName($executable)
        worker_mode = '--zifile-worker'
        separate_worker_present = $false
        archive_entry = 'hello.txt'
        exit_code = 0
        passed = $true
    } | ConvertTo-Json -Compress
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
