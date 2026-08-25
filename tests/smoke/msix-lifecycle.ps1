param(
    [Parameter(Mandatory)][string]$BaselinePackage,
    [Parameter(Mandatory)][string]$UpgradePackage,
    [ValidateSet('x64', 'arm64')]
    [Parameter(Mandatory)][string]$Architecture,
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
    [Parameter(Mandatory)][string]$BaselineVersion,
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
    [Parameter(Mandatory)][string]$UpgradeVersion,
    [Parameter(Mandatory)][string]$IdentityName,
    [Parameter(Mandatory)][string]$Publisher,
    [Parameter(Mandatory)][string]$MinimumWindowsVersion,
    [string]$EvidencePath,
    [string]$RepairHelper,
    [switch]$ConfirmLifecycle
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $ConfirmLifecycle) {
    throw 'MSIX lifecycle testing changes package registration. Pass -ConfirmLifecycle explicitly.'
}
if ([Version]$UpgradeVersion -le [Version]$BaselineVersion) {
    throw "Upgrade version $UpgradeVersion must be greater than baseline version $BaselineVersion."
}
if (-not (Get-Command Reset-AppxPackage -ErrorAction SilentlyContinue)) {
    throw 'Reset-AppxPackage is unavailable on this Windows installation.'
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$packageAudit = Join-Path $repoRoot 'packaging\msix\Test-Package.ps1'
$repairHelperPath = $null
if (-not [string]::IsNullOrWhiteSpace($RepairHelper)) {
    $repairHelperPath = [IO.Path]::GetFullPath($RepairHelper)
    if (-not (Test-Path -LiteralPath $repairHelperPath -PathType Leaf)) {
        throw "MSIX Repair helper does not exist: $repairHelperPath"
    }
}
$baseline = [IO.Path]::GetFullPath($BaselinePackage)
$upgrade = [IO.Path]::GetFullPath($UpgradePackage)
foreach ($package in @($baseline, $upgrade)) {
    if (-not (Test-Path -LiteralPath $package -PathType Leaf)) {
        throw "MSIX package does not exist: $package"
    }
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot 'target\msix-lifecycle.json'
}
$EvidencePath = [IO.Path]::GetFullPath($EvidencePath)
$evidenceParent = Split-Path -Parent $EvidencePath
if (-not (Test-Path -LiteralPath $evidenceParent -PathType Container)) {
    New-Item -ItemType Directory -Path $evidenceParent -Force | Out-Null
}

function Invoke-PackageAudit {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Version
    )

    $json = & $packageAudit `
        -PackagePath $Path `
        -Architecture $Architecture `
        -ExpectedVersion $Version `
        -ExpectedIdentityName $IdentityName `
        -ExpectedPublisher $Publisher `
        -ExpectedMinimumVersion $MinimumWindowsVersion `
        -RequireSignature
    if ($LASTEXITCODE -ne 0) {
        throw "Package audit failed for $(Split-Path -Leaf $Path)."
    }
    $audit = $json | ConvertFrom-Json
    if ($audit.signature_status -cne 'Valid') {
        throw "Package signature is not trusted: $($audit.signature_status)."
    }
    return $audit
}

function Get-InstalledIdentity {
    return @(Get-AppxPackage -Name $IdentityName -ErrorAction SilentlyContinue)
}

function Assert-InstalledVersion {
    param([Parameter(Mandatory)][string]$ExpectedVersion)

    $packages = @(Get-InstalledIdentity)
    if ($packages.Count -ne 1) {
        throw "Expected one installed $IdentityName package, found $($packages.Count)."
    }
    $package = $packages[0]
    if ($package.Version.ToString() -cne $ExpectedVersion) {
        throw "Installed version mismatch: expected $ExpectedVersion, found $($package.Version)."
    }
    if ($package.Publisher -cne $Publisher) {
        throw "Installed publisher mismatch: expected '$Publisher', found '$($package.Publisher)'."
    }
    return $package
}

$baselineAudit = Invoke-PackageAudit -Path $baseline -Version $BaselineVersion
$upgradeAudit = Invoke-PackageAudit -Path $upgrade -Version $UpgradeVersion
if ($baselineAudit.identity -cne $upgradeAudit.identity -or $baselineAudit.publisher -cne $upgradeAudit.publisher) {
    throw 'Baseline and upgrade packages do not share the same identity and publisher.'
}

$preExisting = @(Get-InstalledIdentity)
if ($preExisting.Count -gt 0) {
    throw "Refusing to modify an existing $IdentityName installation. Use a clean test account or machine."
}

$events = [Collections.Generic.List[object]]::new()
$primaryError = $null
$cleanupError = $null
$installedPackage = $null
$passed = $false
$repairStatus = 'not-probed'
$repairDataPreserved = $null
$repairSentinel = $null

try {
    Add-AppxPackage -Path $baseline -ForceApplicationShutdown -ErrorAction Stop
    $installedPackage = Assert-InstalledVersion -ExpectedVersion $BaselineVersion
    $events.Add([ordered]@{ step = 'install'; version = $BaselineVersion; passed = $true })

    $installedCli = Join-Path $installedPackage.InstallLocation 'ZiFile\zifile.exe'
    if (-not (Test-Path -LiteralPath $installedCli -PathType Leaf)) {
        throw "Installed CLI was not found: $installedCli"
    }
    & $installedCli --help | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Installed CLI help failed with exit code $LASTEXITCODE."
    }
    $events.Add([ordered]@{ step = 'installed-cli'; passed = $true })

    Add-AppxPackage -Path $upgrade -ForceApplicationShutdown -ErrorAction Stop
    $installedPackage = Assert-InstalledVersion -ExpectedVersion $UpgradeVersion
    $events.Add([ordered]@{ step = 'upgrade'; version = $UpgradeVersion; passed = $true })

    if ($null -ne $repairHelperPath) {
        $probeScript = Join-Path $PSScriptRoot '..\helpers\msix-repair\Invoke-Probe.ps1'
        $probeJson = & $probeScript -HelperPath $repairHelperPath
        $probe = $probeJson | ConvertFrom-Json
        if ($probe.schema_version -ne 1 -or $probe.operation -cne 'probe') {
            throw 'MSIX Repair helper returned an unexpected probe schema.'
        }

        if ($probe.repair_supported) {
            $packageLocalState = Join-Path $env:LOCALAPPDATA "Packages\$($installedPackage.PackageFamilyName)\LocalState"
            New-Item -ItemType Directory -Path $packageLocalState -Force | Out-Null
            $repairSentinel = Join-Path $packageLocalState 'zifile-lifecycle-repair-sentinel.txt'
            $sentinelValue = [Guid]::NewGuid().ToString('N')
            Set-Content -LiteralPath $repairSentinel -Value $sentinelValue -Encoding utf8

            $repairJson = & $repairHelperPath --package-full-name $installedPackage.PackageFullName
            if ($LASTEXITCODE -ne 0) {
                throw "MSIX Repair failed with exit code $LASTEXITCODE`: $($repairJson -join [Environment]::NewLine)"
            }
            $repair = $repairJson | ConvertFrom-Json
            if ($repair.schema_version -ne 1 -or -not $repair.succeeded) {
                throw "MSIX Repair helper reported failure: $($repairJson -join [Environment]::NewLine)"
            }
            $installedPackage = Assert-InstalledVersion -ExpectedVersion $UpgradeVersion
            $repairDataPreserved = (
                (Test-Path -LiteralPath $repairSentinel -PathType Leaf) -and
                ((Get-Content -Raw -LiteralPath $repairSentinel).Trim() -ceq $sentinelValue)
            )
            if (-not $repairDataPreserved) {
                throw 'MSIX Repair did not preserve the package LocalState sentinel.'
            }
            $repairStatus = 'passed'
            $events.Add([ordered]@{
                step = 'repair'
                version = $UpgradeVersion
                passed = $true
                semantics = 'preserve-application-data'
                local_state_preserved = $true
            })
        }
        else {
            $repairStatus = 'unsupported'
            $events.Add([ordered]@{
                step = 'repair'
                passed = $null
                skipped = $true
                reason = 'PackageDeploymentManager reports RepairPackage unsupported'
            })
        }
    }

    Reset-AppxPackage -Package $installedPackage.PackageFullName -Confirm:$false -ErrorAction Stop
    $installedPackage = Assert-InstalledVersion -ExpectedVersion $UpgradeVersion
    if ($null -ne $repairSentinel -and (Test-Path -LiteralPath $repairSentinel)) {
        throw 'Reset-AppxPackage retained the package LocalState sentinel unexpectedly.'
    }
    $events.Add([ordered]@{
        step = 'reset'
        version = $UpgradeVersion
        passed = $true
        semantics = 'restore-initial-configuration'
    })
    $passed = $true
}
catch {
    $primaryError = $_.Exception.Message
}
finally {
    try {
        $remaining = @(Get-InstalledIdentity)
        foreach ($package in $remaining) {
            $remainingVersion = $package.Version.ToString()
            if (
                $package.Publisher -cne $Publisher -or
                $remainingVersion -notin @($BaselineVersion, $UpgradeVersion)
            ) {
                throw "Refusing to remove unexpected package $($package.PackageFullName)."
            }
            Remove-AppxPackage -Package $package.PackageFullName -Confirm:$false -ErrorAction Stop
        }
        if (@(Get-InstalledIdentity).Count -ne 0) {
            throw "$IdentityName remained installed after Remove-AppxPackage."
        }
        $events.Add([ordered]@{ step = 'uninstall'; passed = $true })
    }
    catch {
        $cleanupError = $_.Exception.Message
        $passed = $false
    }
}

$evidence = [ordered]@{
    schema_version = 1
    generated_at_utc = [DateTime]::UtcNow.ToString('o')
    identity = $IdentityName
    publisher = $Publisher
    architecture = $Architecture
    baseline = [ordered]@{
        version = $BaselineVersion
        sha256 = $baselineAudit.sha256
    }
    upgrade = [ordered]@{
        version = $UpgradeVersion
        sha256 = $upgradeAudit.sha256
    }
    existing_installation_refused = $true
    repair = [ordered]@{
        status = $repairStatus
        semantics = 'preserve-application-data'
        local_state_preserved = $repairDataPreserved
    }
    reset_semantics = 'restore-initial-configuration-and-remove-package-data'
    events = $events
    primary_error = $primaryError
    cleanup_error = $cleanupError
    passed = $passed -and [string]::IsNullOrEmpty($primaryError) -and [string]::IsNullOrEmpty($cleanupError)
}
$evidenceJson = $evidence | ConvertTo-Json -Depth 7
Set-Content -LiteralPath $EvidencePath -Value $evidenceJson -Encoding utf8
$evidenceJson

if (-not [string]::IsNullOrEmpty($cleanupError)) {
    throw "MSIX lifecycle cleanup failed: $cleanupError"
}
if (-not [string]::IsNullOrEmpty($primaryError)) {
    throw "MSIX lifecycle test failed: $primaryError"
}
