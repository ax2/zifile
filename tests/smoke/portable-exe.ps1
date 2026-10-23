param(
    [Parameter(Mandatory)][string]$ExecutablePath,
    [ValidateSet('x64', 'arm64')]
    [string]$Architecture = 'x64',
    [switch]$SkipExecution,
    [ValidateRange(5, 120)][int]$TimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$executable = [IO.Path]::GetFullPath($ExecutablePath)
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Portable executable does not exist: $executable"
}

function Get-PeMachine {
    param([Parameter(Mandatory)][string]$Path)

    $stream = [IO.File]::OpenRead($Path)
    try {
        $reader = [IO.BinaryReader]::new($stream)
        try {
            if ($reader.ReadUInt16() -ne 0x5A4D) {
                throw "Not a PE executable: $Path"
            }
            $stream.Position = 0x3C
            $peOffset = $reader.ReadUInt32()
            if ($peOffset -gt ($stream.Length - 6)) {
                throw "Invalid PE header offset: $Path"
            }
            $stream.Position = $peOffset
            if ($reader.ReadUInt32() -ne 0x00004550) {
                throw "Invalid PE signature: $Path"
            }
            return $reader.ReadUInt16()
        }
        finally {
            $reader.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

$expectedMachine = if ($Architecture -eq 'x64') { 0x8664 } else { 0xAA64 }
$machine = Get-PeMachine -Path $executable
if ($machine -ne $expectedMachine) {
    throw ('Portable executable architecture mismatch: expected {0} for {1}, found 0x{2:X4}.' -f
        $Architecture, $executable, $machine)
}

$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd(
    [IO.Path]::DirectorySeparatorChar,
    [IO.Path]::AltDirectorySeparatorChar
)
$temporaryRoot = Join-Path $temporaryBase "zifile-portable-smoke-$([Guid]::NewGuid().ToString('N'))"
$portableExecutable = Join-Path $temporaryRoot 'zifile.exe'
$workerExecutable = Join-Path $temporaryRoot 'zifile-worker.exe'
$localAppData = Join-Path $temporaryRoot 'localappdata'
$sourceRoot = Join-Path $temporaryRoot 'source'
$sourceFile = Join-Path $sourceRoot 'hello.txt'
$backgroundRoot = Join-Path $temporaryRoot 'background'
$backgroundFile = Join-Path $backgroundRoot 'background.txt'
$quickCreateArchive = Join-Path $sourceRoot 'hello.zip'
$quickCreateHereArchive = Join-Path $backgroundRoot 'background.zip'
$archivePath = Join-Path $temporaryRoot 'sample.zip'
$extractRoot = Join-Path $temporaryRoot 'extracted'

if (-not $temporaryRoot.StartsWith($temporaryBase + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to create the portable smoke fixture outside the system temporary directory: $temporaryRoot"
}

function Invoke-StandaloneWorker {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][hashtable]$Payload,
        [Parameter(Mandatory)][string]$OperationName
    )

    $request = [ordered]@{
        version = 3
        payload = $Payload
    } | ConvertTo-Json -Depth 8 -Compress

    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Executable
    $startInfo.ArgumentList.Add('--zifile-worker')
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo

    if (-not $process.Start()) {
        throw "The standalone portable executable did not start for $OperationName."
    }
    $process.StandardInput.WriteLine($request)
    $process.StandardInput.Close()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $process.Kill($true)
        $process.WaitForExit()
        throw "The standalone $OperationName operation did not finish within $TimeoutSeconds seconds."
    }
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    $exitCode = $process.ExitCode
    $process.Dispose()

    if ($exitCode -ne 0) {
        throw "The standalone $OperationName operation exited with code $exitCode`: $stderr"
    }

    return @(
        $stdout -split '\r?\n' |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_ | ConvertFrom-Json }
    )
}

function Invoke-DesktopCommand {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][string]$OperationName,
        [Parameter(Mandatory)][string]$LocalAppData
    )

    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Executable
    foreach ($argument in $Arguments) {
        $startInfo.ArgumentList.Add($argument)
    }
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Environment['LOCALAPPDATA'] = $LocalAppData
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo

    if (-not $process.Start()) {
        throw "The standalone desktop executable did not start for $OperationName."
    }
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $process.Kill($true)
        $process.WaitForExit()
        throw "The standalone $OperationName operation did not finish within $TimeoutSeconds seconds."
    }
    $stderr = $stderrTask.GetAwaiter().GetResult()
    $exitCode = $process.ExitCode
    $process.Dispose()

    if ($exitCode -ne 0) {
        throw "The standalone $OperationName operation exited with code $exitCode`: $stderr"
    }
}

try {
    New-Item -ItemType Directory -Path $sourceRoot, $backgroundRoot, $localAppData -Force | Out-Null
    Copy-Item -LiteralPath $executable -Destination $portableExecutable

    if (Test-Path -LiteralPath $workerExecutable) {
        throw 'The standalone portable smoke directory unexpectedly contains a separate Worker executable.'
    }

    $settingsDirectory = Join-Path $localAppData 'ZiFile'
    New-Item -ItemType Directory -Path $settingsDirectory -Force | Out-Null
    [IO.File]::WriteAllText(
        (Join-Path $settingsDirectory 'settings.conf'),
        "language=en-US`ntheme=dark`ndefault_create_format=zip`ndefault_compression_level=6`n"
    )

    if ($SkipExecution) {
        [ordered]@{
            schema_version = 1
            executable = [IO.Path]::GetFileName($executable)
            architecture = $Architecture
            pe_machine = ('0x{0:X4}' -f $machine)
            worker_mode = '--zifile-worker'
            separate_worker_present = $false
            execution_skipped = $true
            operations = @()
            round_trip_verified = $false
            passed = $true
        } | ConvertTo-Json -Compress
        return
    }

    $expectedContents = 'ZiFile standalone create-list-extract smoke test'
    [IO.File]::WriteAllText($sourceFile, $expectedContents)
    [IO.File]::WriteAllText($backgroundFile, 'ZiFile folder-background quick-create smoke test')

    Invoke-DesktopCommand `
        -Executable $portableExecutable `
        -Arguments @('--quick-create', $sourceFile) `
        -OperationName 'quick-create' `
        -LocalAppData $localAppData
    if (-not (Test-Path -LiteralPath $quickCreateArchive -PathType Leaf)) {
        throw 'The standalone desktop executable did not create the selected-source quick-create archive.'
    }

    Invoke-DesktopCommand `
        -Executable $portableExecutable `
        -Arguments @('--quick-create-here', $backgroundRoot) `
        -OperationName 'quick-create-here' `
        -LocalAppData $localAppData
    if (-not (Test-Path -LiteralPath $quickCreateHereArchive -PathType Leaf)) {
        throw 'The standalone desktop executable did not create the folder-background quick-create archive.'
    }

    $createEvents = Invoke-StandaloneWorker -Executable $portableExecutable -OperationName 'create' -Payload @{
        operation = 'create'
        sources = @($sourceFile)
        destination = $archivePath
        format = 'Zip'
        compression_level = 6
        password = $null
    }
    if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf) -or
        @($createEvents | Where-Object { $_.payload.event -eq 'summary' }).Count -ne 1) {
        throw 'The standalone portable executable did not create the ZIP archive.'
    }

    $events = Invoke-StandaloneWorker -Executable $portableExecutable -OperationName 'list' -Payload @{
        operation = 'list'
        archive = $archivePath
        password = $null
    }
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

    foreach ($quickArchive in @($quickCreateArchive, $quickCreateHereArchive)) {
        $quickEvents = Invoke-StandaloneWorker -Executable $portableExecutable -OperationName 'quick-create-list' -Payload @{
            operation = 'list'
            archive = $quickArchive
            password = $null
        }
        $quickEntryPaths = @(
            $quickEvents |
                Where-Object { $_.payload.event -eq 'archive_entry' } |
                ForEach-Object { [string]$_.payload.entry.path }
        )
        if ($quickEntryPaths.Count -lt 1) {
            throw "The quick-create archive has no readable entries: $quickArchive"
        }
    }

    $extractEvents = Invoke-StandaloneWorker -Executable $portableExecutable -OperationName 'extract' -Payload @{
        operation = 'extract'
        archive = $archivePath
        destination = $extractRoot
        conflict = 'Rename'
        limits = @{
            max_entries = 1000000
            max_expanded_bytes = 549755813888
            max_expansion_ratio = 1000
            max_path_depth = 128
        }
        password = $null
        selected_paths = $null
    }
    $extractedFile = Join-Path $extractRoot 'hello.txt'
    if (@($extractEvents | Where-Object { $_.payload.event -eq 'summary' }).Count -ne 1 -or
        -not (Test-Path -LiteralPath $extractedFile -PathType Leaf) -or
        [IO.File]::ReadAllText($extractedFile) -cne $expectedContents) {
        throw 'The standalone portable executable did not reproduce the source through create and extract.'
    }

    [ordered]@{
        schema_version = 1
        executable = [IO.Path]::GetFileName($executable)
        architecture = $Architecture
        pe_machine = ('0x{0:X4}' -f $machine)
            worker_mode = '--zifile-worker'
            separate_worker_present = $false
            archive_entry = 'hello.txt'
            operations = @('create', 'list', 'extract', 'quick-create', 'quick-create-here')
            round_trip_verified = $true
            passed = $true
    } | ConvertTo-Json -Compress
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
