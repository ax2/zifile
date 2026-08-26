[CmdletBinding()]
param(
    [string]$StoreDirectory = $PSScriptRoot,
    [string]$DocumentationOutput,
    [switch]$Live,
    [ValidateRange(1, 12)][int]$Attempts = 6,
    [ValidateRange(0, 30)][int]$RetryDelaySeconds = 5
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($DocumentationOutput) -and -not $Live) {
    throw 'Specify -DocumentationOutput, -Live, or both.'
}

$expected = [ordered]@{
    'zh-CN' = [ordered]@{
        url = 'https://ax2.github.io/zifile/product/privacy/'
        marker = '默认遥测'
    }
    'en-US' = [ordered]@{
        url = 'https://ax2.github.io/zifile/en/product/privacy/'
        marker = 'default telemetry'
    }
}
$outputRoot = if ([string]::IsNullOrWhiteSpace($DocumentationOutput)) {
    $null
}
else {
    [IO.Path]::GetFullPath($DocumentationOutput)
}
$results = @()

foreach ($locale in $expected.Keys) {
    $listingPath = Join-Path $StoreDirectory "listing.$locale.json"
    if (-not (Test-Path -LiteralPath $listingPath -PathType Leaf)) {
        throw "Missing Store listing: $listingPath"
    }
    $listing = Get-Content -Raw -LiteralPath $listingPath | ConvertFrom-Json
    $policy = $expected[$locale]
    if ($listing.privacy_policy_url -cne $policy.url) {
        throw "$locale privacy_policy_url must be the deployed ZiFile policy route '$($policy.url)'."
    }

    $uri = [Uri]$policy.url
    $sitePrefix = '/zifile/'
    if (-not $uri.AbsolutePath.StartsWith($sitePrefix, [StringComparison]::Ordinal)) {
        throw "$locale privacy policy URL is outside the ZiFile Pages base path."
    }
    $relativeRoute = $uri.AbsolutePath.Substring($sitePrefix.Length).TrimEnd('/')
    $localValidated = $false
    if ($outputRoot) {
        $segments = @($relativeRoute -split '/' | Where-Object { $_ }) + 'index.html'
        $pagePath = [IO.Path]::Combine([string[]](@($outputRoot) + $segments))
        $resolvedPage = [IO.Path]::GetFullPath($pagePath)
        if (-not $resolvedPage.StartsWith($outputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Resolved privacy page escaped the documentation output directory.'
        }
        if (-not (Test-Path -LiteralPath $resolvedPage -PathType Leaf)) {
            throw "$locale privacy policy route was not generated: $resolvedPage"
        }
        $html = Get-Content -Raw -LiteralPath $resolvedPage
        if (-not $html.Contains('ZiFile', [StringComparison]::Ordinal) -or
            -not $html.Contains($policy.marker, [StringComparison]::OrdinalIgnoreCase)) {
            throw "$locale generated privacy policy is missing its product or privacy marker."
        }
        $localValidated = $true
    }

    $liveValidated = $false
    if ($Live) {
        $lastError = $null
        for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
            try {
                $response = Invoke-WebRequest -Uri $policy.url -MaximumRedirection 5
                if ([int]$response.StatusCode -ne 200) {
                    throw "HTTP $([int]$response.StatusCode)"
                }
                if (-not $response.Content.Contains('ZiFile', [StringComparison]::Ordinal) -or
                    -not $response.Content.Contains($policy.marker, [StringComparison]::OrdinalIgnoreCase)) {
                    throw 'deployed page is missing its product or privacy marker'
                }
                $liveValidated = $true
                break
            }
            catch {
                $lastError = $_.Exception.Message
                if ($attempt -lt $Attempts -and $RetryDelaySeconds -gt 0) {
                    Start-Sleep -Seconds $RetryDelaySeconds
                }
            }
        }
        if (-not $liveValidated) {
            throw "$locale public privacy policy failed after $Attempts attempts: $lastError"
        }
    }

    $results += [ordered]@{
        locale = $locale
        url = $policy.url
        generated_page_validated = $localValidated
        public_page_validated = $liveValidated
    }
}

[pscustomobject]@{
    schema_version = 1
    policies = $results
    generated_pages_validated = [bool]$outputRoot
    public_pages_validated = [bool]$Live
} | ConvertTo-Json -Depth 4
