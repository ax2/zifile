param(
    [string]$ExecutablePath = 'target\x86_64-pc-windows-msvc\release\zifile-desktop-accessible.exe',
    [string]$FixturePath,
    [ValidateRange(1000, 1000000)]
    [int]$EntryCount = 100000,
    [ValidateRange(1, 20)]
    [int]$Iterations = 5,
    [ValidateRange(5, 1000)]
    [int]$SampleMilliseconds = 25,
    [ValidateRange(5, 120)]
    [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$entriesPerPage = 500

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not ('ZiFilePerf.ProcessTreeSampler' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;

namespace ZiFilePerf
{
    public sealed class ProcessTreeSampler : IDisposable
    {
        private const uint TH32CS_SNAPPROCESS = 0x00000002;
        private static readonly IntPtr InvalidHandle = new IntPtr(-1);

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
        private struct PROCESSENTRY32
        {
            public uint dwSize;
            public uint cntUsage;
            public uint th32ProcessID;
            public IntPtr th32DefaultHeapID;
            public uint th32ModuleID;
            public uint cntThreads;
            public uint th32ParentProcessID;
            public int pcPriClassBase;
            public uint dwFlags;
            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)]
            public string szExeFile;
        }

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr CreateToolhelp32Snapshot(uint flags, uint processId);

        [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern bool Process32First(IntPtr snapshot, ref PROCESSENTRY32 entry);

        [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern bool Process32Next(IntPtr snapshot, ref PROCESSENTRY32 entry);

        [DllImport("kernel32.dll")]
        private static extern bool CloseHandle(IntPtr handle);

        private readonly int rootProcessId;
        private readonly int intervalMilliseconds;
        private readonly Thread thread;
        private volatile bool stopping;
        private long peakWorkingSetBytes;
        private long peakPrivateBytes;
        private int peakProcessCount;
        private int sampleCount;

        public ProcessTreeSampler(int rootProcessId, int intervalMilliseconds)
        {
            this.rootProcessId = rootProcessId;
            this.intervalMilliseconds = intervalMilliseconds;
            thread = new Thread(Run);
            thread.IsBackground = true;
            thread.Name = "ZiFile process-tree sampler";
        }

        public long PeakWorkingSetBytes { get { return Interlocked.Read(ref peakWorkingSetBytes); } }
        public long PeakPrivateBytes { get { return Interlocked.Read(ref peakPrivateBytes); } }
        public int PeakProcessCount { get { return Volatile.Read(ref peakProcessCount); } }
        public int SampleCount { get { return Volatile.Read(ref sampleCount); } }

        public void Start()
        {
            thread.Start();
        }

        public void Stop()
        {
            stopping = true;
            if (thread.IsAlive)
            {
                thread.Join(Math.Max(5000, intervalMilliseconds * 4));
            }
        }

        public void Dispose()
        {
            Stop();
        }

        private void Run()
        {
            while (!stopping)
            {
                Sample();
                Thread.Sleep(intervalMilliseconds);
            }
            Sample();
        }

        private void Sample()
        {
            long workingSet = 0;
            long privateBytes = 0;
            int processCount = 0;
            foreach (int processId in GetTreeProcessIds(rootProcessId))
            {
                try
                {
                    using (Process process = Process.GetProcessById(processId))
                    {
                        process.Refresh();
                        workingSet += process.WorkingSet64;
                        privateBytes += process.PrivateMemorySize64;
                        processCount++;
                    }
                }
                catch (ArgumentException) { }
                catch (InvalidOperationException) { }
                catch (System.ComponentModel.Win32Exception) { }
            }
            UpdateMaximum(ref peakWorkingSetBytes, workingSet);
            UpdateMaximum(ref peakPrivateBytes, privateBytes);
            UpdateMaximum(ref peakProcessCount, processCount);
            Interlocked.Increment(ref sampleCount);
        }

        private static HashSet<int> GetTreeProcessIds(int rootProcessId)
        {
            var parentByProcess = new Dictionary<int, int>();
            IntPtr snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if (snapshot != InvalidHandle)
            {
                try
                {
                    var entry = new PROCESSENTRY32();
                    entry.dwSize = (uint)Marshal.SizeOf(typeof(PROCESSENTRY32));
                    if (Process32First(snapshot, ref entry))
                    {
                        do
                        {
                            parentByProcess[(int)entry.th32ProcessID] = (int)entry.th32ParentProcessID;
                            entry.dwSize = (uint)Marshal.SizeOf(typeof(PROCESSENTRY32));
                        }
                        while (Process32Next(snapshot, ref entry));
                    }
                }
                finally
                {
                    CloseHandle(snapshot);
                }
            }

            var ids = new HashSet<int>();
            ids.Add(rootProcessId);
            bool added;
            do
            {
                added = false;
                foreach (KeyValuePair<int, int> pair in parentByProcess)
                {
                    if (ids.Contains(pair.Value) && ids.Add(pair.Key))
                    {
                        added = true;
                    }
                }
            }
            while (added);
            return ids;
        }

        private static void UpdateMaximum(ref long location, long value)
        {
            long current;
            do
            {
                current = Interlocked.Read(ref location);
                if (value <= current) return;
            }
            while (Interlocked.CompareExchange(ref location, value, current) != current);
        }

        private static void UpdateMaximum(ref int location, int value)
        {
            int current;
            do
            {
                current = Volatile.Read(ref location);
                if (value <= current) return;
            }
            while (Interlocked.CompareExchange(ref location, value, current) != current);
        }
    }
}
'@
}

function Resolve-RepoPath {
    param([Parameter(Mandatory)][string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function Get-Percentile {
    param(
        [Parameter(Mandatory)][double[]]$Values,
        [ValidateRange(0.0, 1.0)][double]$Percentile
    )

    $sorted = @($Values | Sort-Object)
    $index = [Math]::Max(0, [Math]::Ceiling($Percentile * $sorted.Count) - 1)
    return [Math]::Round($sorted[$index], 2)
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
        $element = $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
        if ($element) {
            return $element
        }
    }
    return $null
}

function Wait-NamedElement {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$Names,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    do {
        try {
            $element = Find-NamedElement -Root $Root -Names $Names
            if ($element) {
                return $element
            }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        Start-Sleep -Milliseconds 10
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for UI Automation element: $($Names -join ' | ')"
}

function Wait-DocumentText {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$AllOf,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    $documentCondition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Document
    )
    do {
        try {
            $document = $Root.FindFirst(
                [System.Windows.Automation.TreeScope]::Descendants,
                $documentCondition
            )
            if ($document) {
                $textPattern = [System.Windows.Automation.TextPattern]$document.GetCurrentPattern(
                    [System.Windows.Automation.TextPattern]::Pattern
                )
                $text = $textPattern.DocumentRange.GetText(-1)
                $missing = @($AllOf | Where-Object { -not $text.Contains($_) })
                if ($missing.Count -eq 0) {
                    return $text
                }
            }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        catch [System.InvalidOperationException] { }
        Start-Sleep -Milliseconds 10
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for UI Automation document text: $($AllOf -join ' | ')"
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

function Wait-ScrollPercent {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Region,
        [Parameter(Mandatory)][double]$Target,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    $lastPercent = [System.Windows.Automation.ScrollPattern]::NoScroll
    do {
        $pattern = [System.Windows.Automation.ScrollPattern]$Region.GetCurrentPattern(
            [System.Windows.Automation.ScrollPattern]::Pattern
        )
        $lastPercent = $pattern.Current.VerticalScrollPercent
        if ([Math]::Abs($lastPercent - $Target) -lt 0.5) {
            return
        }
        $pattern.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll, $Target)
        Start-Sleep -Milliseconds 10
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for archive table scroll position $Target percent; last UIA value was $lastPercent."
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
                $name = 'folder/{0:D6}.txt' -f $index
                $entry = $archive.CreateEntry($name, [System.IO.Compression.CompressionLevel]::NoCompression)
                $entry.LastWriteTime = [DateTimeOffset]::new(2000, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
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

function Measure-ArchiveBrowser {
    param(
        [Parameter(Mandatory)][string]$Executable,
        [Parameter(Mandatory)][string]$Archive,
        [Parameter(Mandatory)][int]$Iteration,
        [Parameter(Mandatory)][int]$Pages,
        [Parameter(Mandatory)][int]$Entries
    )

    $total = [System.Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $Executable -ArgumentList @("`"$Archive`"") -PassThru
    $sampler = [ZiFilePerf.ProcessTreeSampler]::new($process.Id, $SampleMilliseconds)
    $sampler.Start()
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        do {
            Start-Sleep -Milliseconds 5
            $process.Refresh()
            if ($process.HasExited) {
                throw "$([System.IO.Path]::GetFileName($Executable)) exited during startup with code $($process.ExitCode)."
            }
            if ($process.MainWindowHandle -ne 0 -and $process.Responding) {
                break
            }
        } while ([DateTime]::UtcNow -lt $deadline)
        if ($process.MainWindowHandle -eq 0 -or -not $process.Responding) {
            throw "$([System.IO.Path]::GetFileName($Executable)) did not expose a responding window within $TimeoutSeconds seconds."
        }
        $startupWindowMilliseconds = $total.Elapsed.TotalMilliseconds

        $null = Wait-WindowElement -ProcessId $process.Id -Deadline $deadline
        $englishStatus = "Opened $Entries entries · 0 B expanded"
        $chineseStatus = "已打开 $Entries 个项目 · 展开后 0 B"
        $englishPageOne = "Page 1 / $Pages"
        $chinesePageOne = "页码 1 / $Pages"
        do {
            try {
                $processCondition = [System.Windows.Automation.PropertyCondition]::new(
                    [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
                    [int]$process.Id
                )
                $documentCondition = [System.Windows.Automation.PropertyCondition]::new(
                    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                    [System.Windows.Automation.ControlType]::Document
                )
                $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
                    [System.Windows.Automation.TreeScope]::Children,
                    $processCondition
                )
                for ($windowIndex = 0; $windowIndex -lt $windows.Count; $windowIndex++) {
                    $candidateWindow = $windows.Item($windowIndex)
                    $document = $candidateWindow.FindFirst(
                        [System.Windows.Automation.TreeScope]::Descendants,
                        $documentCondition
                    )
                    if ($document) {
                        $textPattern = [System.Windows.Automation.TextPattern]$document.GetCurrentPattern(
                            [System.Windows.Automation.TextPattern]::Pattern
                        )
                        $documentText = $textPattern.DocumentRange.GetText(-1)
                        $englishReady = $documentText.Contains($englishStatus) -and $documentText.Contains($englishPageOne)
                        $chineseReady = $documentText.Contains($chineseStatus) -and $documentText.Contains($chinesePageOne)
                        if ($englishReady -or $chineseReady) {
                            $window = $candidateWindow
                            break
                        }
                    }
                }
                if ($englishReady -or $chineseReady) {
                    break
                }
            }
            catch [System.Windows.Automation.ElementNotAvailableException] { }
            catch [System.InvalidOperationException] { }
            Start-Sleep -Milliseconds 10
        } while ([DateTime]::UtcNow -lt $deadline)
        if (-not ($englishReady -or $chineseReady)) {
            $tail = if ($documentText) {
                $documentText.Substring([Math]::Max(0, $documentText.Length - 300))
            }
            else {
                '<no document text>'
            }
            throw "Timed out waiting for the archive status and first page in the UI Automation document. Expected '$englishStatus' or '$chineseStatus'; tail: $tail"
        }
        $firstContentMilliseconds = $total.Elapsed.TotalMilliseconds

        $region = Wait-NamedElement -Root $window -Names @('Archive entries', '压缩文件项目') -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $region.SetFocus()
        $scroll = [System.Windows.Automation.ScrollPattern]$region.GetCurrentPattern(
            [System.Windows.Automation.ScrollPattern]::Pattern
        )
        if (-not $scroll.Current.VerticallyScrollable) {
            throw 'Archive table did not expose a vertically scrollable UI Automation pattern.'
        }
        $scrollTimer = [System.Diagnostics.Stopwatch]::StartNew()
        $scroll.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll, 50.0)
        Wait-ScrollPercent -Region $region -Target 50.0 -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $scrollMilliseconds = $scrollTimer.Elapsed.TotalMilliseconds
        $scroll.SetScrollPercent([System.Windows.Automation.ScrollPattern]::NoScroll, 0.0)
        Wait-ScrollPercent -Region $region -Target 0.0 -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))

        $next = Wait-NamedElement -Root $window -Names @('Next', '下一页') -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $invoke = [System.Windows.Automation.InvokePattern]$next.GetCurrentPattern(
            [System.Windows.Automation.InvokePattern]::Pattern
        )
        $pageTimer = [System.Diagnostics.Stopwatch]::StartNew()
        $invoke.Invoke()
        $pageTwoText = if ($englishReady) { "Page 2 / $Pages" } else { "页码 2 / $Pages" }
        $null = Wait-DocumentText -Root $window -AllOf @($pageTwoText) -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $nextPageMilliseconds = $pageTimer.Elapsed.TotalMilliseconds

        $previous = Wait-NamedElement -Root $window -Names @('Previous', '上一页') -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $previousInvoke = [System.Windows.Automation.InvokePattern]$previous.GetCurrentPattern(
            [System.Windows.Automation.InvokePattern]::Pattern
        )
        $previousInvoke.Invoke()
        $pageOneText = if ($englishReady) { $englishPageOne } else { $chinesePageOne }
        $null = Wait-DocumentText -Root $window -AllOf @($pageOneText) -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))

        $sampler.Stop()
        [pscustomobject]@{
            iteration = $Iteration
            startup_window_ms = [Math]::Round($startupWindowMilliseconds, 2)
            first_content_ms = [Math]::Round($firstContentMilliseconds, 2)
            scroll_to_50_percent_ms = [Math]::Round($scrollMilliseconds, 2)
            next_page_ms = [Math]::Round($nextPageMilliseconds, 2)
            peak_process_count = $sampler.PeakProcessCount
            simultaneous_peak_working_set_mib = [Math]::Round($sampler.PeakWorkingSetBytes / 1MB, 2)
            simultaneous_peak_private_mib = [Math]::Round($sampler.PeakPrivateBytes / 1MB, 2)
            memory_samples = $sampler.SampleCount
        }
    }
    finally {
        $sampler.Dispose()
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
    Join-Path ([System.IO.Path]::GetTempPath()) ("zifile-browser-{0}.zip" -f [Guid]::NewGuid().ToString('N'))
}
else {
    Resolve-RepoPath -Path $FixturePath
}
if (-not $generatedFixture -and -not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
    throw "Archive fixture not found: $archivePath"
}

$fixtureTimer = [System.Diagnostics.Stopwatch]::StartNew()
try {
    if ($generatedFixture) {
        New-ArchiveFixture -Path $archivePath -Entries $EntryCount
    }
    $fixtureTimer.Stop()
    $fixtureHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
    $fixtureBytes = (Get-Item -LiteralPath $archivePath).Length
    $pages = [int][Math]::Ceiling($EntryCount / [double]$entriesPerPage)
    if ($pages -lt 2) {
        throw 'The archive fixture must contain at least two pages.'
    }

    $samples = @()
    for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
        $samples += Measure-ArchiveBrowser -Executable $executable -Archive $archivePath -Iteration $iteration -Pages $pages -Entries $EntryCount
    }

    $os = Get-CimInstance Win32_OperatingSystem
    $processor = Get-CimInstance Win32_Processor | Select-Object -First 1
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
            pages = $pages
            bytes = $fixtureBytes
            sha256 = $fixtureHash
            generated = $generatedFixture
            generation_ms = [Math]::Round($fixtureTimer.Elapsed.TotalMilliseconds, 2)
        }
        sampling = [pscustomobject]@{
            iterations = $Iterations
            interval_ms = $SampleMilliseconds
            timeout_seconds = $TimeoutSeconds
        }
        environment = [pscustomobject]@{
            os_caption = $os.Caption
            os_version = $os.Version
            os_build = $os.BuildNumber
            architecture = $env:PROCESSOR_ARCHITECTURE
            logical_processors = $processor.NumberOfLogicalProcessors
            processor = $processor.Name.Trim()
        }
        summary = [pscustomobject]@{
            startup_window_ms_median = Get-Percentile -Values ([double[]]@($samples.startup_window_ms)) -Percentile 0.5
            startup_window_ms_p95 = Get-Percentile -Values ([double[]]@($samples.startup_window_ms)) -Percentile 0.95
            first_content_ms_median = Get-Percentile -Values ([double[]]@($samples.first_content_ms)) -Percentile 0.5
            first_content_ms_p95 = Get-Percentile -Values ([double[]]@($samples.first_content_ms)) -Percentile 0.95
            scroll_to_50_percent_ms_median = Get-Percentile -Values ([double[]]@($samples.scroll_to_50_percent_ms)) -Percentile 0.5
            scroll_to_50_percent_ms_p95 = Get-Percentile -Values ([double[]]@($samples.scroll_to_50_percent_ms)) -Percentile 0.95
            next_page_ms_median = Get-Percentile -Values ([double[]]@($samples.next_page_ms)) -Percentile 0.5
            next_page_ms_p95 = Get-Percentile -Values ([double[]]@($samples.next_page_ms)) -Percentile 0.95
            simultaneous_peak_working_set_mib_max = [Math]::Round(($samples.simultaneous_peak_working_set_mib | Measure-Object -Maximum).Maximum, 2)
            simultaneous_peak_private_mib_max = [Math]::Round(($samples.simultaneous_peak_private_mib | Measure-Object -Maximum).Maximum, 2)
            peak_process_count_max = ($samples.peak_process_count | Measure-Object -Maximum).Maximum
        }
        note = 'First content requires the configured entry count and page label in UI Automation. Scroll uses ScrollPattern to reach 50 percent. UIA observation overhead is included. Peak memory is the maximum simultaneous root-plus-descendant sample, not a sum of per-process peaks.'
        samples = $samples
    }
}
finally {
    if ($generatedFixture -and (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
        [System.IO.File]::Delete($archivePath)
    }
}

$result | ConvertTo-Json -Depth 8
