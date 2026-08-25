$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$publishingPolicy = Join-Path $repoRoot 'packaging\msix\Test-PublishingInputs.ps1'
$packageAudit = Join-Path $repoRoot 'packaging\msix\Test-Package.ps1'
$packageBuild = Join-Path $repoRoot 'packaging\msix\Build-Package.ps1'
$packageLifecycle = Join-Path $repoRoot 'tests\smoke\msix-lifecycle.ps1'
$repairHelper = Join-Path $repoRoot 'tests\helpers\msix-repair\Program.cs'
$repairProject = Join-Path $repoRoot 'tests\helpers\msix-repair\MsixRepair.csproj'

foreach ($script in @($publishingPolicy, $packageAudit, $packageBuild, $packageLifecycle)) {
    $tokens = $null
    $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile(
        $script,
        [ref]$tokens,
        [ref]$errors
    ) | Out-Null
    if ($errors.Count -gt 0) {
        throw "PowerShell parser rejected $(Split-Path $script -Leaf): $($errors -join '; ')"
    }
}

function Get-ExpectedFailure {
    param(
        [Parameter(Mandatory)][scriptblock]$Action,
        [Parameter(Mandatory)][string]$Pattern
    )

    try {
        & $Action | Out-Null
    }
    catch {
        if ($_.Exception.Message -notmatch $Pattern) {
            throw "Expected failure matching '$Pattern', received: $($_.Exception.Message)"
        }
        return $_.Exception.Message
    }
    throw "Expected action to fail with '$Pattern'."
}

$missingFailure = Get-ExpectedFailure -Pattern 'ZIFILE_MSIX_IDENTITY' -Action {
    & $publishingPolicy -IdentityName '' -Publisher ''
}
foreach ($secretName in @(
    'ZIFILE_MSIX_IDENTITY',
    'ZIFILE_MSIX_PUBLISHER',
    'ZIFILE_PFX_BASE64',
    'ZIFILE_PFX_PASSWORD'
)) {
    if ($missingFailure -notmatch [Regex]::Escape($secretName)) {
        throw "Missing publishing input diagnostic omitted $secretName."
    }
}

$null = Get-ExpectedFailure -Pattern 'development MSIX identity' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile.Dev' `
        -Publisher 'CN=ZiCode Official' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}
$null = Get-ExpectedFailure -Pattern 'unsigned development publisher' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Development, OID.2.25.311729368913984317654407730594956997722=1' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}
$accepted = & $publishingPolicy `
    -IdentityName 'ZiCode.ZiFile' `
    -Publisher 'CN=ZiCode Official' `
    -SigningCertificateAvailable `
    -SigningPasswordAvailable |
    ConvertFrom-Json
if (-not $accepted.validated) {
    throw 'Formal publishing inputs were not accepted.'
}

$buildSource = Get-Content -Raw -LiteralPath $packageBuild
if ($buildSource -notmatch [Regex]::Escape("Test-Package.ps1")) {
    throw 'Build-Package.ps1 does not invoke the package auditor.'
}
$releaseSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\release.yml')
$lifecycleWorkflowSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\msix-lifecycle.yml')
if ($releaseSource -notmatch [Regex]::Escape('Test-PublishingInputs.ps1')) {
    throw 'The release workflow does not invoke the publishing input policy.'
}
if ($releaseSource -notmatch [Regex]::Escape('.audit.json')) {
    throw 'The release workflow does not stage MSIX audit evidence.'
}
$manifestSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'packaging\msix\AppxManifest.xml')
foreach ($requiredShellToken in @(
    'windows.comServer',
    'windows.fileExplorerContextMenus',
    'zifile-shell.dll',
    '2F86F25D-3B76-4CD2-8FE8-9D7A2EEFB531'
)) {
    if ($manifestSource -notmatch [Regex]::Escape($requiredShellToken)) {
        throw "The MSIX manifest does not include shell extension token: $requiredShellToken"
    }
}
if ($buildSource -notmatch [Regex]::Escape('zifile_shell.dll')) {
    throw 'Build-Package.ps1 does not stage the architecture-matched shell DLL.'
}
if ($releaseSource -notmatch [Regex]::Escape('zifile-shell-windows-$arch.dll')) {
    throw 'The release workflow does not stage the standalone shell DLL.'
}
$lifecycleSource = Get-Content -Raw -LiteralPath $packageLifecycle
foreach ($requiredLifecycleToken in @(
    'ConfirmLifecycle',
    'RequireSignature',
    'Refusing to modify an existing',
    'Reset-AppxPackage',
    'Remove-AppxPackage',
    'RepairHelper',
    'preserve-application-data',
    'local_state_preserved',
    'reset_semantics'
)) {
    if ($lifecycleSource -notmatch [Regex]::Escape($requiredLifecycleToken)) {
        throw "The MSIX lifecycle gate omits required policy token: $requiredLifecycleToken"
    }
}
foreach ($requiredWorkflowToken in @(
    'baseline_run_id',
    'upgrade_run_id',
    'actions/download-artifact@v7',
    'msix-lifecycle.ps1',
    'ConfirmLifecycle',
    'msix-lifecycle-x64',
    'WindowsAppSDKSelfContained=true',
    'msix-repair-helper',
    '-RepairHelper'
)) {
    if ($lifecycleWorkflowSource -notmatch [Regex]::Escape($requiredWorkflowToken)) {
        throw "The trusted lifecycle workflow omits required token: $requiredWorkflowToken"
    }
}
$repairHelperSource = Get-Content -Raw -LiteralPath $repairHelper
$repairProjectSource = Get-Content -Raw -LiteralPath $repairProject
foreach ($requiredRepairToken in @(
    'PackageDeploymentManager.IsPackageDeploymentFeatureSupported',
    'PackageDeploymentFeature.RepairPackage',
    'RepairPackageAsync',
    '--package-full-name',
    'preserve_application_data'
)) {
    if ($repairHelperSource -notmatch [Regex]::Escape($requiredRepairToken)) {
        throw "The MSIX Repair helper omits required token: $requiredRepairToken"
    }
}
if ($repairProjectSource -notmatch [Regex]::Escape('Microsoft.WindowsAppSDK') -or
    $repairProjectSource -notmatch [Regex]::Escape('1.8.260804001')) {
    throw 'The MSIX Repair helper does not pin its Windows App SDK dependency.'
}

[pscustomobject]@{
    schema_version = 1
    parser_checks = 4
    missing_inputs_rejected = $true
    development_identity_rejected = $true
    unsigned_publisher_rejected = $true
    formal_inputs_accepted = $true
    package_audit_wired = $true
    release_audit_staged = $true
    shell_extension_wired = $true
    lifecycle_gate_wired = $true
    lifecycle_workflow_wired = $true
    repair_helper_wired = $true
} | ConvertTo-Json
