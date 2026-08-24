param(
    [string]$ExecutablePath = 'target\x86_64-pc-windows-msvc\release\zifile-desktop-accessible.exe',
    [ValidateRange(5, 60)]
    [int]$TimeoutSeconds = 20,
    [switch]$ToggleLanguageBeforeTest
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
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetForegroundWindow(IntPtr windowHandle);

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

function Wait-AppElement {
    param(
        [Parameter(Mandatory)][int]$ProcessId,
        [Parameter(Mandatory)][string[]]$Names,
        [Parameter(Mandatory)][DateTime]$Deadline
    )

    do {
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

    do {
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
    throw "Timed out waiting for document text: $($AnyOf -join ' | ')"
}

function Send-AppKey {
    param(
        [Parameter(Mandatory)][string]$Keys,
        [Parameter(Mandatory)][int]$ProcessId
    )

    $null = [ZiFileKeyboard.NativeMethods]::SetForegroundWindow($script:ZiFileWindowHandle)
    if ([ZiFileKeyboard.NativeMethods]::GetForegroundWindow() -ne $script:ZiFileWindowHandle) {
        try {
            $script:ZiFileWindow.SetFocus()
        }
        catch [System.Windows.Automation.ElementNotAvailableException] {
            throw "The ZiFile window became unavailable before sending '$Keys'."
        }
    }
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

$localeRestoreButtonName = $null
$localeRestoreHomeName = $null
$process = Start-Process -FilePath $executable -PassThru
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
        throw "Tab did not reach Home; observed: $($initialFocusSequence -join ' -> ')"
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
        throw 'The password field did not accept keyboard input.'
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
}
