param(
    [string]$Provider,
    [string]$IdentityName,
    [string]$Publisher,
    [switch]$HostAvailable,
    [switch]$ApiKeyAvailable,
    [switch]$ClientCertificateAvailable,
    [switch]$ClientCertificatePasswordAvailable,
    [switch]$KeypairAliasAvailable
)

$ErrorActionPreference = 'Stop'
$missing = [Collections.Generic.List[string]]::new()
if ([string]::IsNullOrWhiteSpace($Provider)) { $missing.Add('ZIFILE_SIGNING_PROVIDER') }
if ([string]::IsNullOrWhiteSpace($IdentityName)) { $missing.Add('ZIFILE_MSIX_IDENTITY') }
if ([string]::IsNullOrWhiteSpace($Publisher)) { $missing.Add('ZIFILE_MSIX_PUBLISHER') }
if (-not $HostAvailable) { $missing.Add('SM_HOST') }
if (-not $ApiKeyAvailable) { $missing.Add('SM_API_KEY') }
if (-not $ClientCertificateAvailable) { $missing.Add('SM_CLIENT_CERT_FILE_B64') }
if (-not $ClientCertificatePasswordAvailable) { $missing.Add('SM_CLIENT_CERT_PASSWORD') }
if (-not $KeypairAliasAvailable) { $missing.Add('SM_KEYPAIR_ALIAS') }
if ($missing.Count -gt 0) {
    throw "Cloud signing requires protected production inputs. Missing: $($missing -join ', ')"
}
if ($Provider -ne 'digicert-stm') {
    throw "Unsupported production signing provider: $Provider"
}
if ($IdentityName.EndsWith('.Dev', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Cloud signing cannot use the development MSIX identity.'
}
if ($Publisher -match 'OID\.2\.25\.311729368913984317654407730594956997722=1') {
    throw 'Cloud signing cannot use the unsigned development publisher namespace.'
}

[pscustomobject]@{
    schema_version = 1
    validated = $true
    provider = $Provider
    identity = $IdentityName
    publisher = $Publisher
    private_code_signing_key_exported = $false
    credential_values_disclosed = $false
} | ConvertTo-Json
