param(
    [string]$ExecutablePath = 'target\x86_64-pc-windows-msvc\release\zifile-desktop-accessible.exe',
    [string]$FixturePath,
    [ValidateRange(1000, 1000000)]
    [int]$EntryCount = 100000,
    [ValidateRange(1, 20)]
    [int]$Iterations = 3,
    [ValidateRange(5, 120)]
    [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

function Resolve-RepoPath {
    param([Parameter(Mandatory)][string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function New-ArchiveFixture {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][int]$Entries
    )

    $parent = [System.IO.Path]::GetDirectoryName($Path)
    [System.IO.Directory]::CreateDirectory($parent) | Out-Null
    $stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::ReadWrite,
        [System.IO.FileShare]::None
    )
    try {
        $archive = [System.IO.Compression.ZipArchive]::new(
            $stream,
            [System.IO.Compression.ZipArchiveMode]::Create,
            $false
        )
        try {
            for ($index = 0; $index -lt $Entries; $index++) {
                $entry = $archive.CreateEntry(
                    ('folder/{0:D6}.txt' -f $index),
                    [System.IO.Compression.CompressionLevel]::NoCompression
                )
                $entry.LastWriteTime = [DateTimeOffset]::new(
                    2000,
                    1,
                    1,
                    0,
                    0,
                    0,
                    [TimeSpan]::Zero
                )
            }
        }
        finally {
            $archive.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Wait-WindowElement {
    param(
        [Parameter(Mandatory)][int]$ProcessId,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $ProcessId
    )
    do {
        $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
            [System.Windows.Automation.TreeScope]::Children,
            $condition
        )
        if ($window) {
            return $window
        }
        Start-Sleep -Milliseconds 10
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for process $ProcessId to expose a UI Automation window."
}

function Find-NamedElement {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$Names
    )

    foreach ($name in $Names) {
        $condition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::NameProperty,
            $name
        )
        $element = $Root.FindFirst(
            [System.Windows.Automation.TreeScope]::Descendants,
            $condition
        )
        if ($element -and $element.Current.IsEnabled) {
            return $element
        }
    }
    return $null
}

function Get-DocumentText {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root)

    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Document
    )
    $document = $Root.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
    if (-not $document) {
        return $null
    }
    $pattern = [System.Windows.Automation.TextPattern]$document.GetCurrentPattern(
        [System.Windows.Automation.TextPattern]::Pattern
    )
    return $pattern.DocumentRange.GetText(-1)
}

function Wait-WorkerExit {
    param(
        [Parameter(Mandatory)][int]$ParentProcessId,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    do {
        $workers = @(
            Get-CimInstance Win32_Process -Filter (
                "Name = 'zifile-worker.exe' AND ParentProcessId = $ParentProcessId"
            )
        )
        if ($workers.Count -eq 0) {
            return
        }
        Start-Sleep -Milliseconds 10
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "The cancelled Worker process owned by desktop process $ParentProcessId did not exit."
}

function Measure-LoadCancellation {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string]$Archive,
        [Parameter(Mandatory)][int]$Iteration
    )

    $process = Start-Process -FilePath $Executable -ArgumentList @("`"$Archive`"") -PassThru
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        $null = Wait-WindowElement -ProcessId $process.Id -Deadline $deadline
        $processCondition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
            [int]$process.Id
        )
        $window = $null
        $cancel = $null
        do {
            try {
                $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
                    [System.Windows.Automation.TreeScope]::Children,
                    $processCondition
                )
                for ($index = 0; $index -lt $windows.Count; $index++) {
                    $candidate = $windows.Item($index)
                    $candidateCancel = Find-NamedElement `
                        -Root $candidate `
                        -Names @('Cancel', '取消')
                    if ($candidateCancel) {
                        $window = $candidate
                        $cancel = $candidateCancel
                        break
                    }
                }
                if ($cancel) {
                    break
                }
            }
            catch [System.Windows.Automation.ElementNotAvailableException] { }
            Start-Sleep -Milliseconds 10
        } while ([DateTime]::UtcNow -lt $deadline)
        if (-not $cancel) {
            throw 'The archive load completed or timed out before the enabled Cancel button was observed.'
        }

        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        $invoke = [System.Windows.Automation.InvokePattern]$cancel.GetCurrentPattern(
            [System.Windows.Automation.InvokePattern]::Pattern
        )
        $invoke.Invoke()

        $documentText = $null
        do {
            try {
                $documentText = Get-DocumentText -Root $window
                $englishCancelled = $documentText -and
                    $documentText.Contains('Open failed') -and
                    $documentText.Contains('Cancelled')
                $chineseCancelled = $documentText -and
                    $documentText.Contains('打开失败') -and
                    $documentText.Contains('取消')
                if ($englishCancelled -or $chineseCancelled) {
                    break
                }
            }
            catch [System.Windows.Automation.ElementNotAvailableException] { }
            catch [System.InvalidOperationException] { }
            Start-Sleep -Milliseconds 10
        } while ([DateTime]::UtcNow -lt $deadline)
        $timer.Stop()

        if (-not ($englishCancelled -or $chineseCancelled)) {
            $tail = if ($documentText) {
                $documentText.Substring([Math]::Max(0, $documentText.Length - 300))
            }
            else {
                '<no document text>'
            }
            throw "The UI did not report a completed cancellation; document tail: $tail"
        }

        $openedStatus = "Opened $EntryCount entries"
        $openedStatusZh = "已打开 $EntryCount 个项目"
        if ($documentText.Contains($openedStatus) -or $documentText.Contains($openedStatusZh)) {
            throw 'The archive reached its successful loaded state after cancellation was requested.'
        }
        Wait-WorkerExit `
            -ParentProcessId $process.Id `
            -Deadline ([DateTime]::UtcNow.AddSeconds(5))

        [pscustomobject]@{
            iteration = $Iteration
            cancellation_ack_ms = [Math]::Round($timer.Elapsed.TotalMilliseconds, 2)
            final_status = if ($englishCancelled) { 'Open failed: Cancelled' } else { '打开失败: Cancelled' }
            worker_processes_after_ack = 0
        }
    }
    finally {
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
            $null = $process.WaitForExit(5000)
        }
    }
}

$executable = Resolve-RepoPath -Path $ExecutablePath
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Accessible desktop executable not found: $executable"
}
$worker = Join-Path ([System.IO.Path]::GetDirectoryName($executable)) 'zifile-worker.exe'
if (-not (Test-Path -LiteralPath $worker -PathType Leaf)) {
    throw "Worker executable must be next to the desktop executable: $worker"
}

$generatedFixture = [string]::IsNullOrWhiteSpace($FixturePath)
$archivePath = if ($generatedFixture) {
    Join-Path ([System.IO.Path]::GetTempPath()) (
        'zifile-cancel-{0}.zip' -f [Guid]::NewGuid().ToString('N')
    )
}
else {
    Resolve-RepoPath -Path $FixturePath
}
if (-not $generatedFixture -and -not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
    throw "Archive fixture not found: $archivePath"
}

try {
    if ($generatedFixture) {
        New-ArchiveFixture -Path $archivePath -Entries $EntryCount
    }
    $fixtureHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
    $samples = for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
        Measure-LoadCancellation `
            -Executable $executable `
            -Archive $archivePath `
            -Iteration $iteration
    }
    $values = @($samples | ForEach-Object { $_.cancellation_ack_ms } | Sort-Object)
    $median = $values[[Math]::Floor(($values.Count - 1) / 2)]
    $p95Index = [Math]::Max(0, [Math]::Ceiling(0.95 * $values.Count) - 1)
    $result = [pscustomobject]@{
        schema_version = 1
        measured_at_utc = [DateTime]::UtcNow.ToString('o')
        git_commit = (git -C $repoRoot rev-parse HEAD).Trim()
        git_dirty = @((git -C $repoRoot status --porcelain)).Count -gt 0
        executable = [pscustomobject]@{
            name = [System.IO.Path]::GetFileName($executable)
            sha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash
        }
        fixture = [pscustomobject]@{
            entries = $EntryCount
            bytes = (Get-Item -LiteralPath $archivePath).Length
            sha256 = $fixtureHash
            generated = $generatedFixture
        }
        iterations = $Iterations
        summary = [pscustomobject]@{
            cancellation_ack_median_ms = [Math]::Round($median, 2)
            cancellation_ack_p95_ms = [Math]::Round($values[$p95Index], 2)
        }
        samples = $samples
    }
    $result | ConvertTo-Json -Depth 6
}
finally {
    if ($generatedFixture -and (Test-Path -LiteralPath $archivePath)) {
        Remove-Item -LiteralPath $archivePath -Force
    }
}
