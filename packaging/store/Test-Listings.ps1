[CmdletBinding()]
param(
    [string]$StoreDirectory = $PSScriptRoot
)

$ErrorActionPreference = 'Stop'
$expectedLocales = @('zh-CN', 'en-US')
$requiredTextFields = @(
    'product_name',
    'short_description',
    'description',
    'category',
    'pricing',
    'applicable_license_terms',
    'developed_by',
    'copyright_trademark',
    'support_url',
    'website_url',
    'privacy_policy_url'
)

function Assert-TextLimit {
    param(
        [Parameter(Mandatory)][string]$Name,
        [AllowEmptyString()][string]$Value,
        [Parameter(Mandatory)][int]$Maximum,
        [switch]$Required
    )

    if ($Required -and [string]::IsNullOrWhiteSpace($Value)) {
        throw "$Name is required."
    }
    if ($Value.Length -gt $Maximum) {
        throw "$Name has $($Value.Length) characters; maximum is $Maximum."
    }
}

$listings = foreach ($locale in $expectedLocales) {
    $path = Join-Path $StoreDirectory "listing.$locale.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing Store listing: $path"
    }

    try {
        $listing = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    }
    catch {
        throw "Invalid Store listing JSON '$path': $($_.Exception.Message)"
    }

    if ($listing.schema_version -ne 1) {
        throw "$locale uses unsupported Store listing schema '$($listing.schema_version)'."
    }
    if ($listing.locale -ne $locale) {
        throw "$path declares locale '$($listing.locale)' instead of '$locale'."
    }
    foreach ($field in $requiredTextFields) {
        Assert-TextLimit -Name "$locale.$field" -Value ([string]$listing.$field) -Maximum 10000 -Required
    }

    Assert-TextLimit -Name "$locale.short_description" -Value $listing.short_description -Maximum 270 -Required
    Assert-TextLimit -Name "$locale.description" -Value $listing.description -Maximum 10000 -Required
    Assert-TextLimit -Name "$locale.whats_new" -Value $listing.whats_new -Maximum 1500
    Assert-TextLimit -Name "$locale.developed_by" -Value $listing.developed_by -Maximum 255 -Required
    Assert-TextLimit -Name "$locale.copyright_trademark" -Value $listing.copyright_trademark -Maximum 200 -Required
    Assert-TextLimit -Name "$locale.applicable_license_terms" -Value $listing.applicable_license_terms -Maximum 10000 -Required

    if ($listing.description -match '(?i)<[a-z!/]|https?://|www\.') {
        throw "$locale.description must be plain Store copy without HTML or URLs."
    }
    if ($listing.features.Count -lt 1 -or $listing.features.Count -gt 20) {
        throw "$locale must define between 1 and 20 product features."
    }
    foreach ($feature in $listing.features) {
        Assert-TextLimit -Name "$locale feature" -Value ([string]$feature) -Maximum 200 -Required
        if ([string]$feature -match '^\s*[-*•]') {
            throw "$locale features must not contain bullet markers; Partner Center adds them."
        }
    }

    if ($listing.keywords.Count -lt 1 -or $listing.keywords.Count -gt 7) {
        throw "$locale must define between 1 and 7 keywords."
    }
    $keywordWordCount = 0
    foreach ($keyword in $listing.keywords) {
        Assert-TextLimit -Name "$locale keyword" -Value ([string]$keyword) -Maximum 40 -Required
        $keywordWordCount += @(([string]$keyword -split '\s+') | Where-Object { $_ }).Count
    }
    if ($keywordWordCount -gt 21) {
        throw "$locale keywords contain $keywordWordCount words; maximum is 21."
    }

    if ($listing.minimum_system_requirements.Count -gt 11) {
        throw "$locale has more than 11 minimum system requirements."
    }
    foreach ($requirement in $listing.minimum_system_requirements) {
        Assert-TextLimit -Name "$locale system requirement" -Value ([string]$requirement) -Maximum 200 -Required
    }

    foreach ($urlField in @('support_url', 'website_url', 'privacy_policy_url')) {
        $uri = $null
        if (-not [Uri]::TryCreate([string]$listing.$urlField, [UriKind]::Absolute, [ref]$uri) -or
            $uri.Scheme -ne 'https') {
            throw "$locale.$urlField must be an absolute HTTPS URL."
        }
    }

    $listing
}

$reference = $listings[0]
foreach ($listing in $listings | Select-Object -Skip 1) {
    foreach ($sharedField in @('schema_version', 'product_name', 'applicable_license_terms', 'developed_by')) {
        if ($listing.$sharedField -ne $reference.$sharedField) {
            throw "Store listings disagree on shared field '$sharedField'."
        }
    }
    if ($listing.features.Count -ne $reference.features.Count) {
        throw 'Localized Store listings must contain the same number of features.'
    }
}

[pscustomobject]@{
    schema_version = 1
    validated = $true
    locales = $expectedLocales
    listing_count = $listings.Count
    feature_count_per_locale = $reference.features.Count
    privacy_policy_urls = @($listings.privacy_policy_url)
} | ConvertTo-Json -Depth 3
