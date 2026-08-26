param(
    [string]$IdentityName,
    [string]$Publisher,
    [switch]$RequireConfigured
)

$ErrorActionPreference = 'Stop'
$identityMissing = [string]::IsNullOrWhiteSpace($IdentityName)
$publisherMissing = [string]::IsNullOrWhiteSpace($Publisher)
if ($identityMissing -and $publisherMissing) {
    if ($RequireConfigured) {
        throw 'Formal publishing requires Repository Variables ZIFILE_MSIX_IDENTITY and ZIFILE_MSIX_PUBLISHER.'
    }
    [ordered]@{
        schema_version = 1
        configured = $false
        formal_identity = $false
    } | ConvertTo-Json
    return
}
if ($identityMissing -or $publisherMissing) {
    throw 'Partner Center Identity and Publisher must be configured together.'
}
if ($IdentityName -ne $IdentityName.Trim() -or $Publisher -ne $Publisher.Trim()) {
    throw 'Partner Center Identity and Publisher cannot contain leading or trailing whitespace.'
}
if ($IdentityName.Length -lt 3 -or $IdentityName.Length -gt 50 -or
    $IdentityName -notmatch '^[A-Za-z0-9.-]+$') {
    throw 'Package Identity Name must contain 3-50 alphanumeric, period, or dash characters.'
}
if ($IdentityName.EndsWith('.Dev', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Partner Center publishing cannot use the development MSIX identity.'
}
if ($Publisher.Length -gt 8192 -or $Publisher -match '[\x00-\x1F\x7F]') {
    throw 'Package Publisher is outside the X.500 distinguished-name boundary.'
}
if ($Publisher -match 'OID\.2\.25\.311729368913984317654407730594956997722=1') {
    throw 'Partner Center publishing cannot use the unsigned development publisher namespace.'
}
try {
    $distinguishedName = [Security.Cryptography.X509Certificates.X500DistinguishedName]::new($Publisher)
    if ([string]::IsNullOrWhiteSpace($distinguishedName.Name)) {
        throw 'empty distinguished name'
    }
}
catch {
    throw "Package Publisher is not a valid X.500 distinguished name: $($_.Exception.Message)"
}

[ordered]@{
    schema_version = 1
    configured = $true
    formal_identity = $true
    identity = $IdentityName
    publisher = $Publisher
    source = 'partner-center-product-identity'
} | ConvertTo-Json
