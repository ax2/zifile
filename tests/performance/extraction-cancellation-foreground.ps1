param(
    [Parameter(Mandatory)][string]$ExecutablePath,
    [string]$FixturePath,
    [ValidateRange(16, 1024)][int]$EntryCount = 128,
    [ValidateRange(65536, 4194304)][int]$EntryBytes = 1048576,
    [ValidateRange(5, 120)][int]$TimeoutSeconds = 45
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

function Resolve-RepoPath {
    param([Parameter(Mandatory)][string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) { return [System.IO.Path]::GetFullPath($Path) }
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Path))
}

function New-ArchiveFixture {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][int]$Entries,
        [Parameter(Mandatory)][int]$BytesPerEntry
    )
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
            $chunk = [byte[]]::new(65536)
            for ($index = 0; $index -lt $Entries; $index++) {
                $entry = $archive.CreateEntry(
                    ('payload/{0:D4}.bin' -f $index),
                    [System.IO.Compression.CompressionLevel]::NoCompression
                )
                $entry.LastWriteTime = [DateTimeOffset]::new(2000, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
                $writer = $entry.Open()
                try {
                    $remaining = $BytesPerEntry
                    while ($remaining -gt 0) {
                        $count = [Math]::Min($remaining, $chunk.Length)
                        $writer.Write($chunk, 0, $count)
                        $remaining -= $count
                    }
                }
                finally { $writer.Dispose() }
            }
        }
        finally { $archive.Dispose() }
    }
    finally { $stream.Dispose() }
}

function Wait-WindowElement {
    param([Parameter(Mandatory)][int]$ProcessId, [Parameter(Mandatory)][DateTime]$Deadline)
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $ProcessId
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
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$Names,
        [Parameter(Mandatory)][DateTime]$Deadline
    )
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

function Wait-DocumentAnyText {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$AnyOf,
        [Parameter(Mandatory)][DateTime]$Deadline
    )
    do {
        try {
            $text = Get-DocumentText -Root $Root
            if ($text -and @($AnyOf | Where-Object { $text.Contains($_) }).Count -gt 0) { return $text }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        catch [System.InvalidOperationException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for document text: $($AnyOf -join ' | ')"
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
    throw 'Worker process remained after extraction cancellation.'
}

$executable = Resolve-RepoPath -Path $ExecutablePath
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw "Desktop executable not found: $executable" }
$worker = Join-Path ([System.IO.Path]::GetDirectoryName($executable)) 'zifile-worker.exe'
if (-not (Test-Path -LiteralPath $worker -PathType Leaf)) { throw "Worker executable must be next to the desktop executable: $worker" }

$generatedFixture = [string]::IsNullOrWhiteSpace($FixturePath)
$archive = if ($generatedFixture) {
    Join-Path ([System.IO.Path]::GetTempPath()) ('zifile-extract-cancel-{0}.zip' -f [Guid]::NewGuid().ToString('N'))
} else { Resolve-RepoPath -Path $FixturePath }
if (-not $generatedFixture -and -not (Test-Path -LiteralPath $archive -PathType Leaf)) { throw "Archive fixture not found: $archive" }

$destination = [System.IO.Path]::Combine(
    [System.IO.Path]::GetDirectoryName($archive),
    [System.IO.Path]::GetFileNameWithoutExtension($archive)
)
$process = $null
try {
    if ($generatedFixture) { New-ArchiveFixture -Path $archive -Entries $EntryCount -BytesPerEntry $EntryBytes }
    $process = Start-Process -FilePath $executable -ArgumentList @('--extract-here', "`"$archive`"") -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $window = Wait-WindowElement -ProcessId $process.Id -Deadline $deadline
    Wait-DocumentAnyText -Root $window -AnyOf @('Extracting to', '正在解压到') -Deadline $deadline | Out-Null
    $cancelButton = Wait-Button -Root $window -Names @('Cancel', '取消') -Deadline $deadline
    ([System.Windows.Automation.InvokePattern]$cancelButton.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )).Invoke()
    $cancelledText = Wait-DocumentAnyText -Root $window -AnyOf @(
        'Extraction failed: Cancelled',
        '解压失败: Cancelled'
    ) -Deadline $deadline
    Wait-WorkersGone -ParentProcessId $process.Id -Deadline $deadline

    $files = if (Test-Path -LiteralPath $destination -PathType Container) {
        @(Get-ChildItem -LiteralPath $destination -Recurse -File -Force)
    } else { @() }
    $partial = @($files | Where-Object { $_.Length -ne $EntryBytes })
    if ($partial.Count -gt 0) {
        throw "Found $($partial.Count) output files whose length is not the complete entry size."
    }

    [pscustomobject]@{
        schema_version = 1
        measured_at_utc = [DateTime]::UtcNow.ToString('o')
        executable = [System.IO.Path]::GetFileName($executable)
        fixture_entries = $EntryCount
        fixture_entry_bytes = $EntryBytes
        fixture_generated = $generatedFixture
        active_cancelled = $true
        cancellation_status_observed = if ($cancelledText.Contains('Extraction failed')) { 'Extraction failed: Cancelled' } else { '解压失败: Cancelled' }
        committed_full_files = $files.Count
        partial_output_files = 0
        partial_output_files_verified = $true
        worker_processes_after_cancel = 0
        passed = $true
    } | ConvertTo-Json -Depth 5
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $null = $process.WaitForExit(5000)
    }
    if ($generatedFixture) {
        if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive -Force }
        if (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Recurse -Force }
    }
}
