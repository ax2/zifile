param(
    [string]$ExecutablePath = 'target\x86_64-pc-windows-msvc\release\zifile-desktop-accessible.exe',
    [ValidateRange(5, 60)]
    [int]$TimeoutSeconds = 20,
    [switch]$ToggleLanguageBeforeTest,
    [switch]$SkipArchiveWorkflow
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms

if (-not ('ZiFileKeyboard.NativeMethods' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

namespace ZiFileKeyboard
{
    public static class NativeMethods
    {
        [DllImport("user32.dll")]
        public static extern IntPtr GetForegroundWindow();
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

function New-ArchiveFixture {
    param([Parameter(Mandatory)][string]$Path)

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
            foreach ($item in @(
                @{ Name = 'alpha.txt'; Content = 'alpha' },
                @{ Name = 'nested/beta.txt'; Content = 'beta' }
            )) {
                $entry = $archive.CreateEntry(
                    $item.Name,
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
                $writer = [System.IO.StreamWriter]::new(
                    $entry.Open(),
                    [System.Text.UTF8Encoding]::new($false)
                )
                try {
                    $writer.Write($item.Content)
                }
                finally {
                    $writer.Dispose()
                }
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

function Get-AppWindows {
    param([Parameter(Mandatory)][int]$ProcessId)

    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
        $ProcessId
    )
    return [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        $condition
    )
}

function Find-NamedElement {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$Names
    )

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
        $element = $Root.FindFirst(
            [System.Windows.Automation.TreeScope]::Descendants,
            $condition
        )
        if ($element) {
            return $element
        }
    }
    return $null
}

function Find-Control {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][System.Windows.Automation.ControlType]$ControlType
    )

    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        $ControlType
    )
    return $Root.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
}

function Assert-AppProcessRunning {
    if (-not $script:ZiFileProcess) {
        return
    }

    $script:ZiFileProcess.Refresh()
    if ($script:ZiFileProcess.HasExited) {
        throw "ZiFile exited unexpectedly during '$script:KeyboardPhase' with exit code $($script:ZiFileProcess.ExitCode)."
    }
}

function Wait-AppElement {
    param(
        [Parameter(Mandatory)][int]$ProcessId,
        [Parameter(Mandatory)][string[]]$Names,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    do {
        Assert-AppProcessRunning
        $windows = @(Get-AppWindows -ProcessId $ProcessId)
        for ($index = 0; $index -lt $windows.Count; $index++) {
            $window = $windows[$index]
            try {
                $element = Find-NamedElement -Root $window -Names $Names
                if ($element) {
                    return [pscustomobject]@{ Window = $window; Element = $element }
                }
            }
            catch [System.Windows.Automation.ElementNotAvailableException] { }
        }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for UI Automation element: $($Names -join ' | ')"
}

function Wait-NamedControl {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$Names,
        [Parameter(Mandatory)][System.Windows.Automation.ControlType]$ControlType,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    $condition = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            $ControlType
        ),
        [System.Windows.Automation.OrCondition]::new(
            @($Names | ForEach-Object {
                    [System.Windows.Automation.PropertyCondition]::new(
                        [System.Windows.Automation.AutomationElement]::NameProperty,
                        $_
                    )
                })
        )
    )
    do {
        Assert-AppProcessRunning
        try {
            $element = $Root.FindFirst(
                [System.Windows.Automation.TreeScope]::Descendants,
                $condition
            )
            if ($element) {
                return $element
            }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for named control: $($Names -join ' | ')"
}

function Restore-LocaleSetting {
    param(
        [Parameter(Mandatory)][int]$ProcessId,
        [Parameter(Mandatory)][string]$ButtonName,
        [Parameter(Mandatory)][string]$HomeName,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    $language = Wait-AppElement `
        -ProcessId $ProcessId `
        -Names @($ButtonName) `
        -Deadline $Deadline
    $invoke = [System.Windows.Automation.InvokePattern]$language.Element.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )
    $invoke.Invoke()
    $null = Wait-AppElement `
        -ProcessId $ProcessId `
        -Names @($HomeName) `
        -Deadline $Deadline
}

function Wait-Control {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][System.Windows.Automation.ControlType]$ControlType,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    do {
        try {
            $element = Find-Control -Root $Root -ControlType $ControlType
            if ($element) {
                return $element
            }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for control type: $($ControlType.ProgrammaticName)"
}

function Get-DocumentText {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root)

    $document = Find-Control `
        -Root $Root `
        -ControlType ([System.Windows.Automation.ControlType]::Document)
    if (-not $document) {
        return $null
    }
    $pattern = [System.Windows.Automation.TextPattern]$document.GetCurrentPattern(
        [System.Windows.Automation.TextPattern]::Pattern
    )
    return $pattern.DocumentRange.GetText(-1)
}

function Wait-DocumentText {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string[]]$AnyOf,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    $text = $null
    do {
        Assert-AppProcessRunning
        try {
            $text = Get-DocumentText -Root $Root
            foreach ($expected in $AnyOf) {
                if ($text -and $text.Contains($expected)) {
                    return $text
                }
            }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        catch [System.InvalidOperationException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    $diagnostic = if ($text) {
        $normalized = ($text -replace '[\r\n]+', ' ').Trim()
        if ($normalized.Length -gt 600) { $normalized.Substring(0, 600) + '…' } else { $normalized }
    }
    else {
        '<no document text>'
    }
    throw "Timed out waiting for document text: $($AnyOf -join ' | '); observed: $diagnostic"
}

function Wait-DocumentTextAbsent {
    param(
        [Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Root,
        [Parameter(Mandatory)][string]$Forbidden,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    $text = $null
    do {
        Assert-AppProcessRunning
        try {
            $text = Get-DocumentText -Root $Root
            if ($text -and -not $text.Contains($Forbidden)) {
                return $text
            }
        }
        catch [System.Windows.Automation.ElementNotAvailableException] { }
        catch [System.InvalidOperationException] { }
        Start-Sleep -Milliseconds 20
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "Timed out waiting for document text to disappear: $Forbidden"
}

function Send-AppKey {
    param(
        [Parameter(Mandatory)][string]$Keys,
        [Parameter(Mandatory)][int]$ProcessId
    )

    Assert-AppProcessRunning
    if ([ZiFileKeyboard.NativeMethods]::GetForegroundWindow() -ne $script:ZiFileWindowHandle) {
        throw "Refusing to send '$Keys' because ZiFile is not foreground during '$script:KeyboardPhase'."
    }
    [System.Windows.Forms.SendKeys]::SendWait($Keys)
}

function Wait-Focus {
    param(
        [Parameter(Mandatory)][int]$ProcessId,
        [string[]]$Names,
        [System.Windows.Automation.ControlType]$ControlType,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    do {
        Assert-AppProcessRunning
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        if (
            $focused -and
            [ZiFileKeyboard.NativeMethods]::GetForegroundWindow() -eq $script:ZiFileWindowHandle
        ) {
            $nameMatches = -not $Names -or $Names -contains $focused.Current.Name
            $typeMatches = -not $ControlType -or $focused.Current.ControlType -eq $ControlType
            if ($nameMatches -and $typeMatches) {
                return $focused
            }
        }
        Start-Sleep -Milliseconds 10
    } while ([DateTime]::UtcNow -lt $Deadline)
    $actual = [System.Windows.Automation.AutomationElement]::FocusedElement
    $actualDescription = if ($actual) {
        "'$($actual.Current.Name)' ($($actual.Current.ControlType.ProgrammaticName))"
    }
    else {
        '<none>'
    }
    throw "Unexpected keyboard focus $actualDescription; expected $($Names -join ' | ') $($ControlType.ProgrammaticName)."
}

function Move-FocusForward {
    param(
        [Parameter(Mandatory)][int]$ProcessId,
        [string[]]$Names,
        [System.Windows.Automation.ControlType]$ControlType,
        [ValidateRange(1, 30)][int]$MaximumTabs = 12,
        [string[]]$ForbiddenNames = @()
    )

    $sequence = @()
    for ($index = 0; $index -lt $MaximumTabs; $index++) {
        Assert-AppProcessRunning
        Send-AppKey -Keys '{TAB}' -ProcessId $ProcessId
        Start-Sleep -Milliseconds 20
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        if (-not $focused) {
            $sequence += '<none>'
            continue
        }
        $name = $focused.Current.Name
        $sequence += "${name}:$($focused.Current.ControlType.ProgrammaticName)"
        if ($ForbiddenNames -contains $name) {
            throw "Keyboard focus reached disabled control '$name'; sequence: $($sequence -join ' -> ')"
        }
        $nameMatches = -not $Names -or $Names -contains $name
        $typeMatches = -not $ControlType -or $focused.Current.ControlType -eq $ControlType
        if ($nameMatches -and $typeMatches) {
            return [pscustomobject]@{ Element = $focused; Sequence = $sequence }
        }
    }
    throw "Tab did not reach the expected control; sequence: $($sequence -join ' -> ')"
}

function Move-FocusBackward {
    param(
        [Parameter(Mandatory)][int]$ProcessId,
        [string[]]$Names,
        [System.Windows.Automation.ControlType]$ControlType,
        [ValidateRange(1, 30)][int]$MaximumTabs = 12,
        [string[]]$ForbiddenNames = @()
    )

    $sequence = @()
    for ($index = 0; $index -lt $MaximumTabs; $index++) {
        Assert-AppProcessRunning
        Send-AppKey -Keys '+{TAB}' -ProcessId $ProcessId
        Start-Sleep -Milliseconds 20
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        if (-not $focused) {
            $sequence += '<none>'
            continue
        }
        $name = $focused.Current.Name
        $sequence += "${name}:$($focused.Current.ControlType.ProgrammaticName)"
        if ($ForbiddenNames -contains $name) {
            throw "Reverse keyboard focus reached disabled control '$name'; sequence: $($sequence -join ' -> ')"
        }
        $nameMatches = -not $Names -or $Names -contains $name
        $typeMatches = -not $ControlType -or $focused.Current.ControlType -eq $ControlType
        if ($nameMatches -and $typeMatches) {
            return [pscustomobject]@{ Element = $focused; Sequence = $sequence }
        }
    }
    throw "Shift+Tab did not reach the expected control; sequence: $($sequence -join ' -> ')"
}

function Get-Value {
    param([Parameter(Mandatory)][System.Windows.Automation.AutomationElement]$Element)

    $pattern = [System.Windows.Automation.ValuePattern]$Element.GetCurrentPattern(
        [System.Windows.Automation.ValuePattern]::Pattern
    )
    return $pattern.Current.Value
}

$executable = Resolve-RepoPath -Path $ExecutablePath
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Accessible desktop executable not found: $executable"
}
$worker = Join-Path ([System.IO.Path]::GetDirectoryName($executable)) 'zifile-worker.exe'
if (-not (Test-Path -LiteralPath $worker -PathType Leaf)) {
    throw "Worker executable must be next to the desktop executable: $worker"
}

$archiveFixturePath = $null
$archiveFixtureHash = $null
$startArguments = @()
if (-not $SkipArchiveWorkflow) {
    $temporaryRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    $archiveFixturePath = Join-Path $temporaryRoot (
        'zifile-keyboard-{0}.zip' -f [Guid]::NewGuid().ToString('N')
    )
    $archiveFixturePath = [System.IO.Path]::GetFullPath($archiveFixturePath)
    if (-not $archiveFixturePath.StartsWith(
        $temporaryRoot,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw 'Refusing to create the keyboard fixture outside the system temporary directory.'
    }
    New-ArchiveFixture -Path $archiveFixturePath
    $archiveFixtureHash = (Get-FileHash -LiteralPath $archiveFixturePath -Algorithm SHA256).Hash
    $startArguments = @("`"$archiveFixturePath`"")
}

$localeRestoreButtonName = $null
$localeRestoreHomeName = $null
$startProcessParameters = @{
    FilePath = $executable
    PassThru = $true
}
if ($startArguments.Count -gt 0) {
    $startProcessParameters.ArgumentList = $startArguments
}
$process = Start-Process @startProcessParameters
$script:ZiFileProcess = $process
try {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $homeResult = Wait-AppElement `
        -ProcessId $process.Id `
        -Names @('Home', '首页') `
        -Deadline $deadline
    $localeSetup = 'existing persisted locale'
    if ($ToggleLanguageBeforeTest) {
        $languageSetup = Wait-AppElement `
            -ProcessId $process.Id `
            -Names @('English', '中文') `
            -Deadline $deadline
        $languageBefore = $languageSetup.Element.Current.Name
        $expectedHome = if ($languageBefore -eq 'English') { 'Home' } else { '首页' }
        $languageInvoke = [System.Windows.Automation.InvokePattern]$languageSetup.Element.GetCurrentPattern(
            [System.Windows.Automation.InvokePattern]::Pattern
        )
        $languageInvoke.Invoke()
        if ($languageBefore -eq 'English') {
            $localeRestoreButtonName = '中文'
            $localeRestoreHomeName = '首页'
        }
        else {
            $localeRestoreButtonName = 'English'
            $localeRestoreHomeName = 'Home'
        }
        $homeResult = Wait-AppElement `
            -ProcessId $process.Id `
            -Names @($expectedHome) `
            -Deadline $deadline
        $localeSetup = "$languageBefore button invoked; expected $expectedHome locale"
    }
    $window = $homeResult.Window
    $script:ZiFileWindow = $window
    $script:ZiFileWindowHandle = [IntPtr]$window.Current.NativeWindowHandle
    if ($script:ZiFileWindowHandle -eq [IntPtr]::Zero) {
        throw 'ZiFile did not expose a native window handle for keyboard activation.'
    }
    $homeButton = $homeResult.Element
    $homeName = $homeButton.Current.Name
    $window.SetFocus()
    $script:KeyboardPhase = 'initial navigation entry'
    $homeFocused = $false
    $initialFocusSequence = @()
    for ($tabIndex = 0; $tabIndex -lt 3; $tabIndex++) {
        Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
        Start-Sleep -Milliseconds 20
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        $initialFocusSequence += if ($focused) {
            "$($focused.Current.Name):$($focused.Current.ControlType.ProgrammaticName):$($focused.Current.ProcessId)"
        }
        else {
            '<none>'
        }
        if (
            $focused -and
            [ZiFileKeyboard.NativeMethods]::GetForegroundWindow() -eq $script:ZiFileWindowHandle -and
            (@('Home', '首页') -contains $focused.Current.Name)
        ) {
            $homeFocused = $true
            break
        }
    }
    if (-not $homeFocused) {
        $homeButton.SetFocus()
        $null = Wait-Focus `
            -ProcessId $process.Id `
            -Names @('Home', '首页') `
            -Deadline $deadline
        $initialFocusSequence += 'explicit Home focus reset after locale change'
        $homeFocused = $true
    }
    $archiveFocus = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Home', '首页') `
        -Deadline $deadline
    Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
    $archiveFocus = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Archive', '压缩文件') `
        -Deadline $deadline
    $archiveName = $archiveFocus.Current.Name
    Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
    $createFocus = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Create', '创建') `
        -Deadline $deadline
    $createName = $createFocus.Current.Name
    Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
    $themeFocus = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Light', 'Dark', '浅色', '深色') `
        -Deadline $deadline
    $themeName = $themeFocus.Current.Name
    Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
    $languageFocus = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('English', '中文') `
        -Deadline $deadline
    $languageName = $languageFocus.Current.Name
    $script:KeyboardPhase = 'reverse navigation and route activation'
    Send-AppKey -Keys '+{TAB}' -ProcessId $process.Id
    $null = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Light', 'Dark', '浅色', '深色') `
        -Deadline $deadline
    Send-AppKey -Keys '+{TAB}' -ProcessId $process.Id
    $null = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Create', '创建') `
        -Deadline $deadline
    Send-AppKey -Keys '+{TAB}' -ProcessId $process.Id
    $null = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Archive', '压缩文件') `
        -Deadline $deadline
    Send-AppKey -Keys '{ENTER}' -ProcessId $process.Id
    $archiveWorkflowCompleted = $false
    $archiveConflictValue = $null
    $reloadActivation = $null
    if (-not $SkipArchiveWorkflow) {
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('Page 1 / 1', '页码 1 / 1') `
            -Deadline $deadline
        $archiveText = Get-DocumentText -Root $window
        if (
            -not $archiveText.Contains('alpha.txt') -or
            -not $archiveText.Contains('nested')
        ) {
            $observedArchiveText = if ($archiveText) {
                ($archiveText -replace '[\r\n]+', ' ').Trim()
            }
            else {
                '<no document text>'
            }
            if ($observedArchiveText.Length -gt 800) {
                $observedArchiveText = $observedArchiveText.Substring(0, 800) + '…'
            }
            throw "The generated two-entry archive did not appear in the archive table. Observed: $observedArchiveText"
        }
        $script:KeyboardPhase = 'archive toolbar and integrity test'
        $null = Move-FocusForward `
            -ProcessId $process.Id `
            -Names @('Open another', '打开其他文件') `
            -MaximumTabs 10
        Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
        $null = Wait-Focus `
            -ProcessId $process.Id `
            -Names @('Test archive', '校验压缩文件') `
            -Deadline $deadline
        Send-AppKey -Keys '{ENTER}' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('Archive is healthy', '压缩文件完好') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))

        $script:KeyboardPhase = 'archive password and reload'
        if ($ToggleLanguageBeforeTest) {
            $openAnother = (Wait-AppElement `
                -ProcessId $process.Id `
                -Names @('Open another', '打开其他文件') `
                -Deadline $deadline).Element
            $openAnother.SetFocus()
            $null = Wait-Focus `
                -ProcessId $process.Id `
                -Names @('Open another', '打开其他文件') `
                -Deadline $deadline
            Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
            $null = Wait-Focus `
                -ProcessId $process.Id `
                -Names @('Test archive', '校验压缩文件') `
                -Deadline $deadline
            Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
            $archivePasswordElement = Wait-Focus `
                -ProcessId $process.Id `
                -Names @('Password (if encrypted)', '密码（如已加密）') `
                -ControlType ([System.Windows.Automation.ControlType]::Edit) `
                -Deadline $deadline
            $archivePasswordFocus = [pscustomobject]@{ Element = $archivePasswordElement }
        }
        else {
            $archivePasswordFocus = Move-FocusForward `
                -ProcessId $process.Id `
                -Names @('Password (if encrypted)', '密码（如已加密）') `
                -ControlType ([System.Windows.Automation.ControlType]::Edit) `
                -MaximumTabs 15
        }
        $archivePassword = $archivePasswordFocus.Element
        if (-not $archivePassword.Current.IsPassword) {
            throw 'The archive password field is not exposed as a protected password control.'
        }
        Send-AppKey -Keys 'archive-test' -ProcessId $process.Id
        Send-AppKey -Keys '^a' -ProcessId $process.Id
        Send-AppKey -Keys '{BACKSPACE}' -ProcessId $process.Id
        Start-Sleep -Milliseconds 100
        if ((Get-Value -Element $archivePassword).Length -ne 0) {
            throw 'Archive password Ctrl+A and Backspace did not clear the field.'
        }
        $archiveText = Get-DocumentText -Root $window
        if (-not ($archiveText.Contains('2 selected') -or
                $archiveText.Contains('2/2 selected') -or
                $archiveText.Contains('2 of 2 files selected') -or
                $archiveText.Contains('2 项已选择'))) {
            $selectionDiagnostic = if ($archiveText) {
                (($archiveText -replace '[\r\n]+', ' ').Trim())
            }
            else {
                '<no document text>'
            }
            if ($selectionDiagnostic.Length -gt 500) {
                $selectionDiagnostic = $selectionDiagnostic.Substring(0, 500) + '…'
            }
            throw "Ctrl+A in the archive password field changed archive selection. Observed: $selectionDiagnostic"
        }
        Start-Sleep -Milliseconds 150
        if ($ToggleLanguageBeforeTest) {
            Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
            $reloadFocus = Wait-Focus `
                -ProcessId $process.Id `
                -Names @('Reload', '重新加载') `
                -Deadline $deadline
        }
        else {
            $reloadFocus = Move-FocusForward `
                -ProcessId $process.Id `
                -Names @('Reload', '重新加载') `
                -MaximumTabs 4
        }
        Send-AppKey -Keys '{ENTER}' -ProcessId $process.Id
        $reloadActivation = 'Enter'
        try {
            $null = Wait-DocumentText `
                -Root $window `
                -AnyOf @('Opening ', '正在打开 ', 'Opened 2 entries', '已打开 2 个项目') `
                -Deadline ([DateTime]::UtcNow.AddSeconds(2))
        }
        catch {
            $null = Wait-Focus `
                -ProcessId $process.Id `
                -Names @('Reload', '重新加载') `
                -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
            Send-AppKey -Keys ' ' -ProcessId $process.Id
            $reloadActivation = 'Space after stable-focus retry'
        }
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('Opened 2 entries', '已打开 2 个项目') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))

        $script:KeyboardPhase = 'archive search and scoped Ctrl+A'
        $searchFocus = Move-FocusForward `
            -ProcessId $process.Id `
            -Names @('Search paths', '搜索路径') `
            -ControlType ([System.Windows.Automation.ControlType]::Edit) `
            -MaximumTabs 15
        $search = $searchFocus.Element
        Send-AppKey -Keys 'beta' -ProcessId $process.Id
        Send-AppKey -Keys '{ENTER}' -ProcessId $process.Id
        Start-Sleep -Milliseconds 100
        $searchFocus = Wait-Focus `
            -ProcessId $process.Id `
            -Names @('Search paths', '搜索路径') `
            -ControlType ([System.Windows.Automation.ControlType]::Edit) `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $search = $searchFocus
        if ((Get-Value -Element $search) -ne 'beta') {
            throw 'Archive search did not accept the committed beta value.'
        }
        $null = Wait-DocumentTextAbsent `
            -Root $window `
            -Forbidden 'alpha.txt' `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('nested/beta.txt', 'nested\beta.txt') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $archiveText = Get-DocumentText -Root $window
        $null = Wait-Focus `
            -ProcessId $process.Id `
            -Names @('Search paths', '搜索路径') `
            -ControlType ([System.Windows.Automation.ControlType]::Edit) `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        Start-Sleep -Milliseconds 100
        Send-AppKey -Keys '^a' -ProcessId $process.Id
        Start-Sleep -Milliseconds 50
        Send-AppKey -Keys '{BACKSPACE}' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('alpha.txt') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $archiveText = Get-DocumentText -Root $window
        if (-not ($archiveText.Contains('2 selected') -or
                $archiveText.Contains('2/2 selected') -or
                $archiveText.Contains('2 of 2 files selected') -or
                $archiveText.Contains('2 项已选择'))) {
            throw 'Ctrl+A in archive search changed archive selection.'
        }

        $script:KeyboardPhase = 'archive selection and conflict policy'
        $selectAllFocus = Move-FocusForward `
            -ProcessId $process.Id `
            -ControlType ([System.Windows.Automation.ControlType]::CheckBox) `
            -MaximumTabs 6
        Send-AppKey -Keys ' ' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('0 selected', '0/2 selected', '0 of 2 files selected', '0/2 项已选择') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $extract = (Wait-AppElement `
            -ProcessId $process.Id `
            -Names @('Extract selected', '解压所选项目') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))).Element
        if ($extract.Current.IsEnabled) {
            throw 'Extract selected remained enabled with zero selected entries.'
        }
        $conflictFocus = Move-FocusForward `
            -ProcessId $process.Id `
            -Names @('Conflict policy', '文件冲突策略') `
            -ControlType ([System.Windows.Automation.ControlType]::ComboBox) `
            -MaximumTabs 5 `
            -ForbiddenNames @('Extract selected', '解压所选项目')
        Send-AppKey -Keys '{HOME}' -ProcessId $process.Id
        Send-AppKey -Keys '{DOWN}' -ProcessId $process.Id
        Start-Sleep -Milliseconds 100
        $archiveConflictValue = Get-Value -Element $conflictFocus.Element
        if (@('Overwrite existing', '覆盖现有文件') -notcontains $archiveConflictValue) {
            throw "Keyboard conflict selection expected overwrite, received '$archiveConflictValue'."
        }

        $tableFocus = Move-FocusForward `
            -ProcessId $process.Id `
            -Names @('Archive entries', '压缩文件项目') `
            -MaximumTabs 5 `
            -ForbiddenNames @('Extract selected', '解压所选项目')
        Send-AppKey -Keys '^a' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('2 selected', '2/2 selected', '2 of 2 files selected', '2 项已选择') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $entryCheckboxFocus = Move-FocusForward `
            -ProcessId $process.Id `
            -ControlType ([System.Windows.Automation.ControlType]::CheckBox) `
            -MaximumTabs 8 `
            -ForbiddenNames @('Extract selected', '解压所选项目')
        Send-AppKey -Keys ' ' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('1 selected', '1/2 selected', '1 of 2 files selected', '1/2 项已选择') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $null = Move-FocusBackward `
            -ProcessId $process.Id `
            -Names @('Archive entries', '压缩文件项目') `
            -MaximumTabs 8
        Send-AppKey -Keys '^a' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('2 selected', '2/2 selected', '2 of 2 files selected', '2 项已选择') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $extractFocus = Move-FocusBackward `
            -ProcessId $process.Id `
            -Names @('Extract selected', '解压所选项目') `
            -MaximumTabs 4
        if (-not $extractFocus.Element.Current.IsEnabled) {
            throw 'Extract selected did not become enabled after table Ctrl+A.'
        }
        $previous = (Wait-AppElement `
            -ProcessId $process.Id `
            -Names @('Previous', '上一页') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))).Element
        $next = (Wait-AppElement `
            -ProcessId $process.Id `
            -Names @('Next', '下一页') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))).Element
        if ($previous.Current.IsEnabled -or $next.Current.IsEnabled) {
            throw 'Single-page archive pagination exposed an enabled direction.'
        }
        $archiveWorkflowCompleted = $true

        $script:KeyboardPhase = 'Ctrl+N transition to create form'
        Send-AppKey -Keys '^n' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('Choose sources, format and compression.', '选择来源、格式与压缩等级。') `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
    }
    else {
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('No archive open', '尚未打开压缩文件') `
            -Deadline $deadline
        Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
        $null = Wait-Focus `
            -ProcessId $process.Id `
            -Names @('Create', '创建') `
            -Deadline $deadline
        Send-AppKey -Keys '{ENTER}' -ProcessId $process.Id
        $null = Wait-DocumentText `
            -Root $window `
            -AnyOf @('Choose sources, format and compression.', '选择来源、格式与压缩等级。') `
            -Deadline $deadline
    }
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $script:KeyboardPhase = 'create form traversal'

    $clear = (Wait-AppElement `
        -ProcessId $process.Id `
        -Names @('Clear', '清空') `
        -Deadline $deadline).Element
    $createAction = (Wait-AppElement `
        -ProcessId $process.Id `
        -Names @('Create archive', '创建压缩文件') `
        -Deadline $deadline).Element
    $cancel = (Wait-AppElement `
        -ProcessId $process.Id `
        -Names @('Cancel', '取消') `
        -Deadline $deadline).Element
    if ($clear.Current.IsEnabled -or $createAction.Current.IsEnabled -or $cancel.Current.IsEnabled) {
        throw 'Idle create-page actions exposed an incorrect enabled state.'
    }
    $disabledActionNames = @(
        $clear.Current.Name,
        $createAction.Current.Name,
        $cancel.Current.Name
    )

    $addFilesFocus = Move-FocusForward `
        -ProcessId $process.Id `
        -Names @('Add files', '添加文件') `
        -MaximumTabs 10 `
        -ForbiddenNames @('Clear', '清空')

    Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
    $addFolderFocus = Wait-Focus `
        -ProcessId $process.Id `
        -Names @('Add folder', '添加文件夹') `
        -Deadline $deadline
    $comboFocus = Move-FocusForward `
        -ProcessId $process.Id `
        -ControlType ([System.Windows.Automation.ControlType]::ComboBox) `
        -MaximumTabs 4 `
        -ForbiddenNames @('Clear', '清空')
    $combo = $comboFocus.Element

    Send-AppKey -Keys '{HOME}' -ProcessId $process.Id
    Send-AppKey -Keys '{DOWN}' -ProcessId $process.Id
    Start-Sleep -Milliseconds 100
    $selectedFormat = Get-Value -Element $combo
    if ($selectedFormat -ne '7z') {
        throw "Keyboard format selection expected '7z', received '$selectedFormat'."
    }

    Send-AppKey -Keys '{TAB}' -ProcessId $process.Id
    $slider = Wait-Focus `
        -ProcessId $process.Id `
        -ControlType ([System.Windows.Automation.ControlType]::Slider) `
        -Deadline $deadline
    $range = [System.Windows.Automation.RangeValuePattern]$slider.GetCurrentPattern(
        [System.Windows.Automation.RangeValuePattern]::Pattern
    )
    $initialLevel = $range.Current.Value
    Send-AppKey -Keys '{RIGHT}' -ProcessId $process.Id
    Start-Sleep -Milliseconds 100
    $incrementedLevel = $range.Current.Value
    if ($initialLevel -ne 6 -or $incrementedLevel -ne 7) {
        throw "Compression keyboard increment expected 6 -> 7, received $initialLevel -> $incrementedLevel."
    }
    Send-AppKey -Keys '{LEFT}' -ProcessId $process.Id
    Start-Sleep -Milliseconds 100
    if ($range.Current.Value -ne 6) {
        throw "Compression keyboard decrement did not restore level 6; received $($range.Current.Value)."
    }

    $passwordFocus = Move-FocusForward `
        -ProcessId $process.Id `
        -ControlType ([System.Windows.Automation.ControlType]::Edit) `
        -MaximumTabs 4 `
        -ForbiddenNames @('Create archive', '创建压缩文件', 'Cancel', '取消')
    $password = $passwordFocus.Element
    if (-not $password.Current.IsPassword) {
        throw 'The create password field is not exposed as a protected password control.'
    }
    Send-AppKey -Keys 'keyboard-test' -ProcessId $process.Id
    Start-Sleep -Milliseconds 100
    $passwordLength = (Get-Value -Element $password).Length
    if ($passwordLength -eq 0) {
        $focusedDescription = try {
            $focusedNow = [System.Windows.Automation.AutomationElement]::FocusedElement
            if ($focusedNow) {
                "'$($focusedNow.Current.Name)' ($($focusedNow.Current.ControlType.ProgrammaticName), enabled=$($focusedNow.Current.IsEnabled), keyboardFocus=$($focusedNow.Current.HasKeyboardFocus))"
            }
            else {
                '<none>'
            }
        }
        catch {
            '<focus unavailable>'
        }
        throw "The password field did not accept keyboard input. Focused: $focusedDescription"
    }
    Send-AppKey -Keys '^a' -ProcessId $process.Id
    Send-AppKey -Keys '{BACKSPACE}' -ProcessId $process.Id
    Start-Sleep -Milliseconds 100
    if ((Get-Value -Element $password).Length -ne 0) {
        throw 'Ctrl+A and Backspace did not clear the password field.'
    }

    $null = Move-FocusBackward `
        -ProcessId $process.Id `
        -ControlType ([System.Windows.Automation.ControlType]::Slider) `
        -MaximumTabs 4 `
        -ForbiddenNames @('Create archive', '创建压缩文件', 'Cancel', '取消')
    $null = Move-FocusBackward `
        -ProcessId $process.Id `
        -ControlType ([System.Windows.Automation.ControlType]::ComboBox) `
        -MaximumTabs 4 `
        -ForbiddenNames @('Create archive', '创建压缩文件', 'Cancel', '取消')

    $script:KeyboardPhase = 'final reverse traversal'
    $null = Move-FocusBackward `
        -ProcessId $process.Id `
        -Names @('Add folder', '添加文件夹') `
        -MaximumTabs 4 `
        -ForbiddenNames @('Clear', '清空')

    $localeRestored = -not $ToggleLanguageBeforeTest
    if ($localeRestoreButtonName) {
        Restore-LocaleSetting `
            -ProcessId $process.Id `
            -ButtonName $localeRestoreButtonName `
            -HomeName $localeRestoreHomeName `
            -Deadline ([DateTime]::UtcNow.AddSeconds($TimeoutSeconds))
        $localeRestoreButtonName = $null
        $localeRestoreHomeName = $null
        $localeRestored = $true
    }

    [pscustomobject]@{
        schema_version = 1
        tested_at_utc = [DateTime]::UtcNow.ToString('o')
        executable = [pscustomobject]@{
            name = [System.IO.Path]::GetFileName($executable)
            sha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash
        }
        navigation = [pscustomobject]@{
            locale_setup = $localeSetup
            locale_restored = $localeRestored
            forward = @(
                $homeName,
                $archiveName,
                $createName,
                $themeName,
                $languageName
            )
            reverse_theme_language = $true
            archive_and_create_activation = $true
        }
        archive_workflow = [pscustomobject]@{
            tested = -not $SkipArchiveWorkflow
            completed = $archiveWorkflowCompleted
            fixture_entries = if ($SkipArchiveWorkflow) { 0 } else { 2 }
            fixture_sha256 = $archiveFixtureHash
            integrity_test = $archiveWorkflowCompleted
            reload = $archiveWorkflowCompleted
            reload_activation = $reloadActivation
            password_ctrl_a_scoped = $archiveWorkflowCompleted
            search_ctrl_a_scoped = $archiveWorkflowCompleted
            search_seed = if ($archiveWorkflowCompleted) { 'keyboard beta plus Enter composition commit' } else { $null }
            conflict_policy = $archiveConflictValue
            selection = if ($archiveWorkflowCompleted) { '2 -> 0 -> 2 -> 1 -> 2' } else { $null }
            extract_enabled_state = $archiveWorkflowCompleted
            single_page_pagination_disabled = $archiveWorkflowCompleted
        }
        create_form = [pscustomobject]@{
            disabled_actions_skipped = $disabledActionNames
            selected_format = $selectedFormat
            compression_level = '6 -> 7 -> 6'
            password_keyboard_entry_and_clear = $true
            password_value_recorded = $false
            reverse_combo_slider_password = $true
            add_files_button_reached_by_keyboard = $true
            add_folder_button_reached_by_keyboard = $true
            system_dialogs_opened = $false
        }
    } | ConvertTo-Json -Depth 6
}
finally {
    if ($localeRestoreButtonName -and -not $process.HasExited) {
        try {
            Restore-LocaleSetting `
                -ProcessId $process.Id `
                -ButtonName $localeRestoreButtonName `
                -HomeName $localeRestoreHomeName `
                -Deadline ([DateTime]::UtcNow.AddSeconds(5))
        }
        catch {
            Write-Warning "ZiFile test locale restoration failed: $($_.Exception.Message)"
        }
    }
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $null = $process.WaitForExit(5000)
    }
    if ($archiveFixturePath -and (Test-Path -LiteralPath $archiveFixturePath)) {
        Remove-Item -LiteralPath $archiveFixturePath -Force
    }
}
