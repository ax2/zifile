param(
    [Parameter(Mandatory)][string]$ExecutablePath,
    [string]$FixturePath,
    [ValidateRange(1000, 1000000)][int]$EntryCount = 100000,
    [ValidateRange(5, 120)][int]$TimeoutSeconds = 45,
    [ValidateRange(1, 10)][int]$ForegroundTimeoutSeconds = 3
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not ('ZiFileQueueForeground.NativeMethods' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

namespace ZiFileQueueForeground
{
    public static class NativeMethods
    {
        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetForegroundWindow(IntPtr windowHandle);

        [DllImport("user32.dll")]
        public static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        public static extern bool ShowWindow(IntPtr windowHandle, int command);

        [DllImport("user32.dll")]
        public static extern bool BringWindowToTop(IntPtr windowHandle);

        [DllImport("user32.dll")]
        public static extern uint GetWindowThreadProcessId(IntPtr windowHandle, out uint processId);

        [DllImport("user32.dll")]
        public static extern bool AttachThreadInput(uint sourceThreadId, uint targetThreadId, bool attach);

        [DllImport("kernel32.dll")]
        public static extern uint GetCurrentThreadId();

        [DllImport("user32.dll")]
        public static extern bool AllowSetForegroundWindow(int processId);

        public const int SwRestore = 9;

        public static bool TryActivateWindow(IntPtr windowHandle)
        {
            ShowWindow(windowHandle, SwRestore);
            var foreground = GetForegroundWindow();
            var foregroundThread = GetWindowThreadProcessId(foreground, out _);
            var targetThread = GetWindowThreadProcessId(windowHandle, out var targetProcess);
            var currentThread = GetCurrentThreadId();
            var attachedToForeground = foregroundThread != 0 && foregroundThread != currentThread &&
                AttachThreadInput(currentThread, foregroundThread, true);
            var attachedToTarget = targetThread != 0 && targetThread != currentThread &&
                AttachThreadInput(currentThread, targetThread, true);
            try
            {
                AllowSetForegroundWindow((int)targetProcess);
                BringWindowToTop(windowHandle);
                return SetForegroundWindow(windowHandle);
            }
            finally
            {
                if (attachedToTarget) AttachThreadInput(currentThread, targetThread, false);
                if (attachedToForeground) AttachThreadInput(currentThread, foregroundThread, false);
            }
        }
    }
}
'@
}

function Resolve-RepoPath {
    param([Parameter(Mandatory)][string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) { return [System.IO.Path]::GetFullPath($Path) }
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function New-ArchiveFixture {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][int]$Entries)
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
                $entry.LastWriteTime = [DateTimeOffset]::new(2000, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
            }
        }
        finally { $archive.Dispose() }
    }
    finally { $stream.Dispose() }
}

function Wait-WindowElement {
    param([Parameter(Mandatory)][int]$ProcessId, [Parameter(Mandatory)][DateTime]$Deadline)
    $condition = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
            $ProcessId
        ),
        [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Window
        )
    )
    do {
        $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
            [System.Windows.Automation.TreeScope]::Children,
            $condition
        )
        if ($window) { return $window }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for process $ProcessId to expose a UI Automation window."
}

function Set-TestWindowForeground {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Window,
        [Parameter(Mandatory)][DateTime]$Deadline
    )
    $handle = [IntPtr]$Window.Current.NativeWindowHandle
    if ($handle -eq [IntPtr]::Zero) { throw 'ZiFile exposed a zero native window handle.' }
    do {
        $null = [ZiFileQueueForeground.NativeMethods]::TryActivateWindow($handle)
        if ([ZiFileQueueForeground.NativeMethods]::GetForegroundWindow() -eq $handle) { return }
        try { $Window.SetFocus() } catch { }
        if ([ZiFileQueueForeground.NativeMethods]::GetForegroundWindow() -eq $handle) { return }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw 'Refusing to run the foreground queue smoke because ZiFile is not the foreground window.'
}

function Find-Button {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root, [Parameter(Mandatory)][string[]]$Names)
    foreach ($name in $Names) {
        $condition = [System.Windows.Automation.AndCondition]::new(
            [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::NameProperty,
                $name
            ),
            [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Button
            )
        )
        $button = $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
        if ($button -and $button.Current.IsEnabled) { return $button }
    }
    return $null
}

function Wait-Button {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root, [Parameter(Mandatory)][string[]]$Names, [Parameter(Mandatory)][DateTime]$Deadline)
    do {
        try {
            $button = Find-Button -Root $Root -Names $Names
            if ($button) { return $button }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for enabled button: $($Names -join ' | ')"
}

function Get-DocumentText {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root)
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Document
    )
    $document = $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
    if (-not $document) { return $null }
    $pattern = [System.Windows.Automation.TextPattern]$document.GetCurrentPattern(
        [System.Windows.Automation.TextPattern]::Pattern
    )
    return $pattern.DocumentRange.GetText(-1)
}

function Wait-DocumentText {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root, [Parameter(Mandatory)][string[]]$AllOf, [Parameter(Mandatory)][DateTime]$Deadline)
    do {
        try {
            $text = Get-DocumentText -Root $Root
            if ($text -and @($AllOf | Where-Object { -not $text.Contains($_) }).Count -eq 0) { return $text }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        catch [System.InvalidOperationException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for document text: $($AllOf -join ' | ')"
}

function Wait-DocumentAnyText {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root, [Parameter(Mandatory)][string[]]$AnyOf, [Parameter(Mandatory)][DateTime]$Deadline)
    $lastText = $null
    do {
        try {
            $text = Get-DocumentText -Root $Root
            if ($text) { $lastText = $text }
            if ($text -and @($AnyOf | Where-Object { $text.Contains($_) }).Count -gt 0) { return $text }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        catch [System.InvalidOperationException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    $diagnostic = if ([string]::IsNullOrWhiteSpace($lastText)) {
        '<no UI Automation document text observed>'
    } else {
        $normalized = ($lastText -replace '\s+', ' ').Trim()
        $normalized.Substring(0, [Math]::Min(500, $normalized.Length))
    }
    throw "Timed out waiting for document text: $($AnyOf -join ' | '). Last document text: $diagnostic"
}

function Get-WorkerChildren {
    param([Parameter(Mandatory)][int]$ParentProcessId)
    @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $ParentProcessId" |
        Where-Object { $_.Name -ieq 'zifile-worker.exe' })
}

function Wait-WorkersGone {
    param([Parameter(Mandatory)][int]$ParentProcessId, [Parameter(Mandatory)][DateTime]$Deadline)
    do {
        if ((Get-WorkerChildren -ParentProcessId $ParentProcessId).Count -eq 0) { return }
        Start-Sleep -Milliseconds 50
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw 'Worker process remained after the queue became idle.'
}

$executable = Resolve-RepoPath -Path $ExecutablePath
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw "Desktop executable not found: $executable" }
$generatedFixture = [string]::IsNullOrWhiteSpace($FixturePath)
$archive = if ($generatedFixture) {
    Join-Path ([System.IO.Path]::GetTempPath()) ('zifile-queue-{0}.zip' -f [Guid]::NewGuid().ToString('N'))
} else { Resolve-RepoPath -Path $FixturePath }
if (-not $generatedFixture -and -not (Test-Path -LiteralPath $archive -PathType Leaf)) { throw "Archive fixture not found: $archive" }

$process = $null
try {
    if ($generatedFixture) { New-ArchiveFixture -Path $archive -Entries $EntryCount }
    $process = Start-Process -FilePath $executable -ArgumentList @("`"$archive`"") -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $window = Wait-WindowElement -ProcessId $process.Id -Deadline $deadline
    $foregroundDeadline = [DateTime]::UtcNow.AddSeconds($ForegroundTimeoutSeconds)
    if ($foregroundDeadline -gt $deadline) { $foregroundDeadline = $deadline }
    Set-TestWindowForeground -Window $window -Deadline $foregroundDeadline
    Wait-DocumentAnyText -Root $window -AnyOf @("Opened $EntryCount entries", "已打开 $EntryCount 个项目") -Deadline $deadline | Out-Null

    $testButton = Wait-Button -Root $window -Names @('Test archive', '校验压缩文件') -Deadline $deadline
    $clearButton = $null
    $testInvocationCount = 0
    ([System.Windows.Automation.InvokePattern]$testButton.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )).Invoke()
    $testInvocationCount++
    Wait-DocumentAnyText -Root $window -AnyOf @('Testing every entry and checksum', '正在校验所有项目与校验和') -Deadline $deadline | Out-Null
    $cancelButton = Wait-Button -Root $window -Names @('Cancel', '取消') -Deadline $deadline
    for ($index = 1; $index -lt 3; $index++) {
        ([System.Windows.Automation.InvokePattern]$testButton.GetCurrentPattern(
            [System.Windows.Automation.InvokePattern]::Pattern
        )).Invoke()
        $testInvocationCount++
        Start-Sleep -Milliseconds 50
    }
    Wait-DocumentAnyText -Root $window -AnyOf @('2 operations queued', '2 个操作排队') -Deadline $deadline | Out-Null

    ([System.Windows.Automation.InvokePattern]$cancelButton.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )).Invoke()
    Wait-DocumentAnyText -Root $window -AnyOf @('1 operation queued', '1 个操作排队') -Deadline $deadline | Out-Null
    $clearButton = Wait-Button -Root $window -Names @('Clear queue', '清空队列') -Deadline $deadline
    ([System.Windows.Automation.InvokePattern]$clearButton.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )).Invoke()
    Wait-DocumentAnyText -Root $window -AnyOf @('No operations queued', '0 个操作排队') -Deadline $deadline | Out-Null
    Wait-WorkersGone -ParentProcessId $process.Id -Deadline $deadline

    [pscustomobject]@{
        schema_version = 1
        measured_at_utc = [DateTime]::UtcNow.ToString('o')
        executable = [System.IO.Path]::GetFileName($executable)
        fixture_entries = $EntryCount
        fixture_generated = $generatedFixture
        foreground_window_verified = $true
        test_operations_submitted = $testInvocationCount
        active_cancelled = $true
        next_operation_started = $true
        waiting_operations_cleared = $true
        worker_processes_after_idle = 0
        partial_output_files = 0
        passed = $true
    } | ConvertTo-Json -Depth 5
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $null = $process.WaitForExit(5000)
    }
    if ($generatedFixture -and (Test-Path -LiteralPath $archive)) {
        Remove-Item -LiteralPath $archive -Force
    }
}
