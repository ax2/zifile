param(
    [string]$IdentityName,
    [string]$Publisher,
    [string]$PublisherDisplayName,
    [switch]$RequireConfigured
)

$ErrorActionPreference = 'Stop'
$identityMissing = [string]::IsNullOrWhiteSpace($IdentityName)
$publisherMissing = [string]::IsNullOrWhiteSpace($Publisher)
$publisherDisplayNameMissing = [string]::IsNullOrWhiteSpace($PublisherDisplayName)
if ($identityMissing -and $publisherMissing -and $publisherDisplayNameMissing) {
    if ($RequireConfigured) {
        throw 'Formal publishing requires Repository Variables ZIFILE_MSIX_IDENTITY, ZIFILE_MSIX_PUBLISHER, and ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME.'
    }
    [ordered]@{
        schema_version = 1
        configured = $false
        formal_identity = $false
    } | ConvertTo-Json
    return
}
if ($identityMissing -or $publisherMissing -or $publisherDisplayNameMissing) {
    throw 'Partner Center Identity, Publisher, and Publisher Display Name must be configured together.'
}
if ($IdentityName -ne $IdentityName.Trim() -or $Publisher -ne $Publisher.Trim() -or
    $PublisherDisplayName -ne $PublisherDisplayName.Trim()) {
    throw 'Partner Center identity fields cannot contain leading or trailing whitespace.'
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
if ($PublisherDisplayName.Length -gt 256 -or $PublisherDisplayName -match '[\x00-\x1F\x7F]') {
    throw 'Publisher Display Name must contain 1-256 printable characters.'
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
    publisher_display_name = $PublisherDisplayName
    source = 'partner-center-product-identity'
} | ConvertTo-Json
