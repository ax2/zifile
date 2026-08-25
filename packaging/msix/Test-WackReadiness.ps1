[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackagePath,
    [Parameter(Mandatory)][string]$AuditPath,
    [string]$AppCertPath = (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\App Certification Kit\appcert.exe'),
    [string]$EvidencePath,
    [switch]$RequireReady
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$package = [IO.Path]::GetFullPath($PackagePath)
$auditFile = [IO.Path]::GetFullPath($AuditPath)
$appCert = [IO.Path]::GetFullPath($AppCertPath)
$issues = [Collections.Generic.List[object]]::new()

function Add-ReadinessIssue {
    param([Parameter(Mandatory)][string]$Code, [Parameter(Mandatory)][string]$Message)
    $issues.Add([pscustomobject]@{ code = $Code; message = $Message })
}

$runningOnWindows = [Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [Runtime.InteropServices.OSPlatform]::Windows
)
if (-not $runningOnWindows) { Add-ReadinessIssue 'windows_required' 'WACK requires Windows.' }

$isAdministrator = $false
if ($runningOnWindows) {
    $principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    $isAdministrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}
if (-not $isAdministrator) {
    Add-ReadinessIssue 'administrator_required' 'WACK must run from the current user interactive administrator session.'
}
$userInteractive = [Environment]::UserInteractive
if (-not $userInteractive) {
    Add-ReadinessIssue 'interactive_session_required' 'WACK requires an interactive user session.'
}

$appCertVersion = $null
if (-not (Test-Path -LiteralPath $appCert -PathType Leaf)) {
    Add-ReadinessIssue 'appcert_missing' "Windows App Certification Kit was not found: $appCert"
}
else {
    $appCertVersion = (Get-Item -LiteralPath $appCert).VersionInfo.FileVersion
}

$packageExists = Test-Path -LiteralPath $package -PathType Leaf
$auditExists = Test-Path -LiteralPath $auditFile -PathType Leaf
if (-not $packageExists) { Add-ReadinessIssue 'package_missing' "MSIX package does not exist: $package" }
if (-not $auditExists) { Add-ReadinessIssue 'audit_missing' "MSIX audit does not exist: $auditFile" }

$audit = $null
if ($auditExists) {
    try { $audit = Get-Content -Raw -LiteralPath $auditFile | ConvertFrom-Json }
    catch { Add-ReadinessIssue 'audit_invalid' "MSIX audit is not valid JSON: $($_.Exception.Message)" }
}

$actualPackageHash = $null
$actualSignatureStatus = $null
if ($packageExists) {
    $actualPackageHash = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash
    $actualSignatureStatus = (Get-AuthenticodeSignature -LiteralPath $package).Status.ToString()
    if ($actualSignatureStatus -cne 'Valid') {
        Add-ReadinessIssue 'package_signature_invalid' "MSIX signature status is $actualSignatureStatus instead of Valid."
    }
}

if ($null -ne $audit) {
    if ($audit.schema_version -ne 2) { Add-ReadinessIssue 'audit_schema_invalid' 'WACK requires a schema v2 MSIX audit.' }
    if ($packageExists -and $audit.package -cne [IO.Path]::GetFileName($package)) {
        Add-ReadinessIssue 'audit_package_mismatch' 'MSIX audit package name does not match the selected package.'
    }
    if ($packageExists -and $audit.sha256 -cne $actualPackageHash) {
        Add-ReadinessIssue 'audit_hash_mismatch' 'MSIX audit SHA-256 does not match the selected package.'
    }
    if (-not $audit.signature_required -or $audit.signature_status -cne 'Valid') {
        Add-ReadinessIssue 'audit_signature_invalid' 'MSIX audit does not prove a required Valid signature.'
    }
    if ([string]$audit.identity -match '(?i)\.Dev$') {
        Add-ReadinessIssue 'development_identity' 'Development MSIX identity cannot enter WACK release certification.'
    }
    if ([string]$audit.publisher -match 'OID\.2\.25\.311729368913984317654407730594956997722=1') {
        Add-ReadinessIssue 'unsigned_publisher' 'Unsigned development publisher namespace cannot enter WACK release certification.'
    }
    if ($audit.minimum_windows_version -cne '10.0.19041.0') {
        Add-ReadinessIssue 'minimum_windows_mismatch' 'Formal ZiFile packages must retain Windows 10 build 19041 minimum support.'
    }
    if ($audit.forbidden_file_count -ne 0) {
        Add-ReadinessIssue 'forbidden_files' 'MSIX audit reports forbidden files.'
    }
    if ($audit.architecture -notin @('x64', 'arm64')) {
        Add-ReadinessIssue 'architecture_invalid' "Unsupported MSIX audit architecture: $($audit.architecture)"
    }
    elseif ($runningOnWindows) {
        $hostArchitecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
        if ($hostArchitecture -cne [string]$audit.architecture) {
            Add-ReadinessIssue 'architecture_host_mismatch' "Package architecture $($audit.architecture) cannot be runtime-certified on $hostArchitecture Windows."
        }
    }
}

$evidence = [pscustomobject]@{
    schema_version = 1
    operation = 'wack-readiness'
    ready = $issues.Count -eq 0
    appcert_path = $appCert
    appcert_version = $appCertVersion
    administrator = $isAdministrator
    user_interactive = $userInteractive
    package = if ($packageExists) { [IO.Path]::GetFileName($package) } else { $null }
    package_sha256 = $actualPackageHash
    package_signature_status = $actualSignatureStatus
    audit = if ($auditExists) { [IO.Path]::GetFileName($auditFile) } else { $null }
    issues = @($issues)
}
$json = $evidence | ConvertTo-Json -Depth 5
if ($EvidencePath) {
    $resolvedEvidence = [IO.Path]::GetFullPath($EvidencePath)
    $evidenceParent = Split-Path -Parent $resolvedEvidence
    if (-not (Test-Path -LiteralPath $evidenceParent -PathType Container)) {
        throw "WACK readiness evidence directory does not exist: $evidenceParent"
    }
    Set-Content -LiteralPath $resolvedEvidence -Value $json -Encoding utf8
}
$json
if ($RequireReady -and -not $evidence.ready) {
    throw "WACK readiness failed: $(@($issues.code) -join ', ')"
}
