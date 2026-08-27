param(
    [Parameter(Mandatory)][string]$ArtifactDirectory,
    [ValidateSet('x64', 'arm64')]
    [Parameter(Mandatory)][string]$Architecture,
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
    [Parameter(Mandatory)][string]$ExpectedVersion,
    [Parameter(Mandatory)][string]$ExpectedIdentityName,
    [Parameter(Mandatory)][string]$ExpectedPublisher,
    [Parameter(Mandatory)][string]$ExpectedPublisherDisplayName,
    [ValidateSet('digicert-stm')]
    [Parameter(Mandatory)][string]$Provider
)

$ErrorActionPreference = 'Stop'
$directory = [IO.Path]::GetFullPath($ArtifactDirectory)
if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
    throw "Signed artifact directory does not exist: $directory"
}

$requiredFiles = @(
    "zifile-desktop-windows-$Architecture.exe",
    "zifile-cli-windows-$Architecture.exe",
    "zifile-worker-windows-$Architecture.exe",
    "zifile-shell-windows-$Architecture.dll",
    "ZiFile-$ExpectedVersion-windows-$Architecture.msix"
)
$signatures = [Collections.Generic.List[object]]::new()
foreach ($name in $requiredFiles) {
    $path = Join-Path $directory $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required signed release artifact is missing: $name"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Release artifact signature is not valid for ${name}: $($signature.Status)"
    }
    if (-not $signature.SignerCertificate -or -not $signature.TimeStamperCertificate) {
        throw "Release artifact lacks a signer or RFC 3161 timestamp certificate: $name"
    }
    if ($signature.SignerCertificate.Subject -ne $ExpectedPublisher) {
        throw "Release artifact signer does not match Publisher for $name."
    }
    $signatures.Add([pscustomobject]@{
        file = $name
        status = $signature.Status.ToString()
        signer_subject = $signature.SignerCertificate.Subject
        signer_thumbprint = $signature.SignerCertificate.Thumbprint
        timestamp_subject = $signature.TimeStamperCertificate.Subject
        timestamp_thumbprint = $signature.TimeStamperCertificate.Thumbprint
    })
}
if (@($signatures.signer_thumbprint | Sort-Object -Unique).Count -ne 1) {
    throw 'Release artifacts were not signed by one certificate.'
}
if ((Get-ChildItem -LiteralPath $directory -File -Filter '*.zip').Count -ne 0) {
    throw 'Signed release artifacts must not contain ZIP files.'
}

$packagePath = Join-Path $directory "ZiFile-$ExpectedVersion-windows-$Architecture.msix"
$packageAuditPath = Join-Path $directory "ZiFile-$ExpectedVersion-windows-$Architecture.audit.json"
& (Join-Path $PSScriptRoot 'Test-Package.ps1') `
    -PackagePath $packagePath `
    -Architecture $Architecture `
    -ExpectedVersion $ExpectedVersion `
    -ExpectedIdentityName $ExpectedIdentityName `
    -ExpectedPublisher $ExpectedPublisher `
    -ExpectedPublisherDisplayName $ExpectedPublisherDisplayName `
    -ExpectedMinimumVersion '10.0.19041.0' `
    -EvidencePath $packageAuditPath `
    -RequireSignature

$evidence = [pscustomobject]@{
    schema_version = 1
    provider = $Provider
    architecture = $Architecture
    expected_version = $ExpectedVersion
    identity = $ExpectedIdentityName
    publisher = $ExpectedPublisher
    publisher_display_name = $ExpectedPublisherDisplayName
    signatures_valid = $true
    timestamped = $true
    artifacts = @($signatures)
}
$signingAuditPath = Join-Path $directory "ZiFile-$ExpectedVersion-windows-$Architecture.signing.json"
$evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $signingAuditPath -Encoding utf8NoBOM

$checksumsPath = Join-Path $directory "SHA256SUMS-$Architecture.txt"
if (Test-Path -LiteralPath $checksumsPath) { Remove-Item -LiteralPath $checksumsPath -Force }
Get-ChildItem -LiteralPath $directory -File |
    Sort-Object Name |
    Get-FileHash -Algorithm SHA256 |
    ForEach-Object { "{0}  {1}" -f $_.Hash.ToLowerInvariant(), (Split-Path $_.Path -Leaf) } |
    Set-Content -LiteralPath $checksumsPath -Encoding utf8NoBOM

$evidence | ConvertTo-Json -Depth 6
