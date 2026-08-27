[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackagePath,
    [Parameter(Mandatory)][string]$AuditPath,
    [Parameter(Mandatory)][string]$ExpectedIdentityName,
    [Parameter(Mandatory)][string]$ExpectedPublisher,
    [Parameter(Mandatory)][string]$ExpectedPublisherDisplayName,
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
$identityPreflight = Join-Path $PSScriptRoot '..\store\Test-PartnerCenterIdentity.ps1'
$formalIdentity = & $identityPreflight `
    -IdentityName $ExpectedIdentityName `
    -Publisher $ExpectedPublisher `
    -PublisherDisplayName $ExpectedPublisherDisplayName `
    -RequireConfigured | ConvertFrom-Json
if (-not $formalIdentity.formal_identity) {
    throw 'WACK readiness requires the formal Partner Center identity tuple.'
}

function Add-ReadinessIssue {
    param([Parameter(Mandatory)][string]$Code, [Parameter(Mandatory)][string]$Message)
    $issues.Add([pscustomobject]@{ code = $Code; message = $Message })
}

function Get-AuditValue {
    param(
        [Parameter(Mandatory)][object]$InputObject,
        [Parameter(Mandatory)][string]$Name
    )
    $property = $InputObject.PSObject.Properties[$Name]
    if ($null -eq $property) { return $null }
    $property.Value
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
    $requiredAuditFields = @(
        'schema_version', 'package', 'sha256', 'signature_required', 'signature_status',
        'identity', 'publisher', 'publisher_display_name', 'minimum_windows_version',
        'forbidden_file_count', 'architecture'
    )
    $missingAuditFields = @($requiredAuditFields | Where-Object {
        $null -eq $audit.PSObject.Properties[$_]
    })
    if ($missingAuditFields.Count -gt 0) {
        Add-ReadinessIssue 'audit_schema_invalid' "MSIX audit is missing required fields: $($missingAuditFields -join ', ')."
    }
    $auditSchemaVersion = Get-AuditValue -InputObject $audit -Name 'schema_version'
    $auditPackage = Get-AuditValue -InputObject $audit -Name 'package'
    $auditSha256 = Get-AuditValue -InputObject $audit -Name 'sha256'
    $auditSignatureRequired = Get-AuditValue -InputObject $audit -Name 'signature_required'
    $auditSignatureStatus = Get-AuditValue -InputObject $audit -Name 'signature_status'
    $auditIdentity = Get-AuditValue -InputObject $audit -Name 'identity'
    $auditPublisher = Get-AuditValue -InputObject $audit -Name 'publisher'
    $auditPublisherDisplayName = Get-AuditValue -InputObject $audit -Name 'publisher_display_name'
    $auditMinimumWindowsVersion = Get-AuditValue -InputObject $audit -Name 'minimum_windows_version'
    $auditForbiddenFileCount = Get-AuditValue -InputObject $audit -Name 'forbidden_file_count'
    $auditArchitecture = Get-AuditValue -InputObject $audit -Name 'architecture'

    if ($auditSchemaVersion -ne 2) { Add-ReadinessIssue 'audit_schema_invalid' 'WACK requires a schema v2 MSIX audit.' }
    if ($packageExists -and $auditPackage -cne [IO.Path]::GetFileName($package)) {
        Add-ReadinessIssue 'audit_package_mismatch' 'MSIX audit package name does not match the selected package.'
    }
    if ($packageExists -and $auditSha256 -cne $actualPackageHash) {
        Add-ReadinessIssue 'audit_hash_mismatch' 'MSIX audit SHA-256 does not match the selected package.'
    }
    if ($auditSignatureRequired -ne $true -or $auditSignatureStatus -cne 'Valid') {
        Add-ReadinessIssue 'audit_signature_invalid' 'MSIX audit does not prove a required Valid signature.'
    }
    if ([string]$auditIdentity -cne $ExpectedIdentityName) {
        Add-ReadinessIssue 'identity_mismatch' 'MSIX audit Identity does not match the formal Partner Center Identity.'
    }
    if ([string]$auditPublisher -cne $ExpectedPublisher) {
        Add-ReadinessIssue 'publisher_mismatch' 'MSIX audit Publisher does not match the formal Partner Center Publisher.'
    }
    if ([string]$auditPublisherDisplayName -cne $ExpectedPublisherDisplayName) {
        Add-ReadinessIssue 'publisher_display_name_mismatch' 'MSIX audit Publisher Display Name does not match the formal Partner Center value.'
    }
    if ([string]$auditPublisher -match 'OID\.2\.25\.311729368913984317654407730594956997722=1') {
        Add-ReadinessIssue 'unsigned_publisher' 'Unsigned development publisher namespace cannot enter WACK release certification.'
    }
    if ($auditMinimumWindowsVersion -cne '10.0.19041.0') {
        Add-ReadinessIssue 'minimum_windows_mismatch' 'Formal ZiFile packages must retain Windows 10 build 19041 minimum support.'
    }
    if ($auditForbiddenFileCount -ne 0) {
        Add-ReadinessIssue 'forbidden_files' 'MSIX audit reports forbidden files.'
    }
    if ($auditArchitecture -notin @('x64', 'arm64')) {
        Add-ReadinessIssue 'architecture_invalid' "Unsupported MSIX audit architecture: $auditArchitecture"
    }
    elseif ($runningOnWindows) {
        $hostArchitecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
        if ($hostArchitecture -cne [string]$auditArchitecture) {
            Add-ReadinessIssue 'architecture_host_mismatch' "Package architecture $auditArchitecture cannot be runtime-certified on $hostArchitecture Windows."
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
    expected_identity = $ExpectedIdentityName
    expected_publisher = $ExpectedPublisher
    expected_publisher_display_name = $ExpectedPublisherDisplayName
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
