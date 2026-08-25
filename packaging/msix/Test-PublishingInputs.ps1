param(
    [string]$IdentityName,
    [string]$Publisher,
    [string]$ReleaseVersion,
    [switch]$SigningCertificateAvailable,
    [switch]$SigningPasswordAvailable
)

$ErrorActionPreference = 'Stop'
$missing = [System.Collections.Generic.List[string]]::new()
if ([string]::IsNullOrWhiteSpace($IdentityName)) { $missing.Add('ZIFILE_MSIX_IDENTITY') }
if ([string]::IsNullOrWhiteSpace($Publisher)) { $missing.Add('ZIFILE_MSIX_PUBLISHER') }
if (-not $SigningCertificateAvailable) { $missing.Add('ZIFILE_PFX_BASE64') }
if (-not $SigningPasswordAvailable) { $missing.Add('ZIFILE_PFX_PASSWORD') }
if ($missing.Count -gt 0) {
    throw "Tagged releases require formal signing and Partner Center identity secrets. Missing: $($missing -join ', ')"
}
if ($IdentityName.EndsWith('.Dev', [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Tagged releases cannot use the development MSIX identity.'
}
if ($Publisher -match 'OID\.2\.25\.311729368913984317654407730594956997722=1') {
    throw 'Tagged releases cannot use the unsigned development publisher namespace.'
}
if (-not [string]::IsNullOrWhiteSpace($ReleaseVersion)) {
    $normalizedVersion = $ReleaseVersion.TrimStart('v')
    if ($normalizedVersion -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
        throw "Release version is not a supported semantic version: $ReleaseVersion"
    }
    if ($normalizedVersion -notmatch '-') {
        throw 'Stable tags require the ADR-0006 cloud-HSM signing integration; the PFX workflow is pre-release scaffolding only.'
    }
}

[pscustomobject]@{
    schema_version = 1
    validated = $true
    identity = $IdentityName
    publisher = $Publisher
    release_version = $ReleaseVersion
    signing_provider = 'pfx-scaffolding'
    signing_certificate_available = [bool]$SigningCertificateAvailable
    signing_password_available = [bool]$SigningPasswordAvailable
} | ConvertTo-Json
