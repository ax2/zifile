param(
    [string[]]$ExecutablePaths = @(
        'target\release\zifile-desktop.exe',
        'target\release\zifile-desktop-accessible.exe'
    ),
    [ValidateRange(1, 20)]
    [int]$Iterations = 5,
    [ValidateRange(250, 10000)]
    [int]$SettleMilliseconds = 1500,
    [ValidateRange(5, 60)]
    [int]$StartupTimeoutSeconds = 20
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))

function Get-DescendantProcessIds {
    param([int]$RootId)

    $rows = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    $null = $ids.Add($RootId)
    do {
        $added = $false
        foreach ($row in $rows) {
            if ($ids.Contains([int]$row.ParentProcessId) -and $ids.Add([int]$row.ProcessId)) {
                $added = $true
            }
        }
    } while ($added)
    return @($ids)
}

function Get-Percentile {
    param(
        [double[]]$Values,
        [ValidateRange(0.0, 1.0)]
        [double]$Percentile
    )

    $sorted = @($Values | Sort-Object)
    $index = [Math]::Max(0, [Math]::Ceiling($Percentile * $sorted.Count) - 1)
    return [Math]::Round($sorted[$index], 2)
}

function Measure-DesktopProcess {
    param(
        [string]$Executable,
        [int]$Iteration
    )

    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $Executable -PassThru
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
        do {
            Start-Sleep -Milliseconds 10
            $process.Refresh()
            if ($process.HasExited) {
                throw "$([System.IO.Path]::GetFileName($Executable)) exited during startup with code $($process.ExitCode)."
            }
            if ($process.MainWindowHandle -ne 0 -and $process.Responding) {
                break
            }
        } while ([DateTime]::UtcNow -lt $deadline)
        if ($process.MainWindowHandle -eq 0 -or -not $process.Responding) {
            throw "$([System.IO.Path]::GetFileName($Executable)) did not expose a responding window within $StartupTimeoutSeconds seconds."
        }
        $startupWindowMilliseconds = $stopwatch.Elapsed.TotalMilliseconds

        Start-Sleep -Milliseconds $SettleMilliseconds
        $ids = Get-DescendantProcessIds -RootId $process.Id
        $processes = @($ids | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue })
        $workingSetBytes = ($processes | Measure-Object WorkingSet64 -Sum).Sum
        $privateBytes = ($processes | Measure-Object PrivateMemorySize64 -Sum).Sum

        [pscustomobject]@{
            iteration = $Iteration
            startup_window_ms = [Math]::Round($startupWindowMilliseconds, 2)
            settled_process_count = $processes.Count
            settled_working_set_mib = [Math]::Round($workingSetBytes / 1MB, 2)
            settled_private_mib = [Math]::Round($privateBytes / 1MB, 2)
        }
    }
    finally {
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
            $null = $process.WaitForExit(5000)
        }
    }
}

$results = @()
foreach ($path in $ExecutablePaths) {
    $executable = if ([System.IO.Path]::IsPathRooted($path)) {
        [System.IO.Path]::GetFullPath($path)
    }
    else {
        [System.IO.Path]::GetFullPath((Join-Path $repoRoot $path))
    }
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Desktop executable not found: $executable"
    }

    $samples = @()
    for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
        $samples += Measure-DesktopProcess -Executable $executable -Iteration $iteration
    }
    $startup = [double[]]@($samples.startup_window_ms)
    $workingSet = [double[]]@($samples.settled_working_set_mib)
    $privateMemory = [double[]]@($samples.settled_private_mib)
    $results += [pscustomobject]@{
        executable = [System.IO.Path]::GetFileName($executable)
        sha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash
        iterations = $Iterations
        settle_milliseconds = $SettleMilliseconds
        summary = [pscustomobject]@{
            startup_window_ms_median = Get-Percentile -Values $startup -Percentile 0.5
            startup_window_ms_p95 = Get-Percentile -Values $startup -Percentile 0.95
            working_set_mib_median = Get-Percentile -Values $workingSet -Percentile 0.5
            working_set_mib_p95 = Get-Percentile -Values $workingSet -Percentile 0.95
            private_mib_median = Get-Percentile -Values $privateMemory -Percentile 0.5
            private_mib_p95 = Get-Percentile -Values $privateMemory -Percentile 0.95
        }
        samples = $samples
    }
}

$os = Get-CimInstance Win32_OperatingSystem
$processor = Get-CimInstance Win32_Processor | Select-Object -First 1
[pscustomobject]@{
    schema_version = 1
    measured_at_utc = [DateTime]::UtcNow.ToString('o')
    git_commit = (git -C $repoRoot rev-parse HEAD).Trim()
    git_dirty = @((git -C $repoRoot status --porcelain)).Count -gt 0
    environment = [pscustomobject]@{
        os_caption = $os.Caption
        os_version = $os.Version
        os_build = $os.BuildNumber
        architecture = $env:PROCESSOR_ARCHITECTURE
        logical_processors = $processor.NumberOfLogicalProcessors
        processor = $processor.Name.Trim()
    }
    note = 'Startup measures time to a responding native window; settled memory sums the desktop root and descendants.'
    results = $results
} | ConvertTo-Json -Depth 8
