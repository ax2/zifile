$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$validator = Join-Path $repoRoot 'packaging\store\Test-Listings.ps1'
$screenshotValidator = Join-Path $repoRoot 'packaging\store\Test-Screenshots.ps1'
$sourceDirectory = Join-Path $repoRoot 'packaging\store'
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$testDirectory = Join-Path $tempRoot "zifile-store-listing-$([Guid]::NewGuid().ToString('N'))"

$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
    $validator,
    [ref]$tokens,
    [ref]$errors
) | Out-Null
if ($errors.Count -gt 0) {
    throw "PowerShell parser rejected Test-Listings.ps1: $($errors -join '; ')"
}
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile($screenshotValidator, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count -gt 0) { throw "PowerShell parser rejected Test-Screenshots.ps1: $($errors -join '; ')" }

function Get-ExpectedFailure {
    param(
        [Parameter(Mandatory)][scriptblock]$Action,
        [Parameter(Mandatory)][string]$Pattern
    )

    try {
        & $Action | Out-Null
    }
    catch {
        if ($_.Exception.Message -notmatch $Pattern) {
            throw "Expected failure matching '$Pattern', received: $($_.Exception.Message)"
        }
        return
    }
    throw "Expected action to fail with '$Pattern'."
}

function Write-TestListing {
    param(
        [Parameter(Mandatory)][string]$Locale,
        [Parameter(Mandatory)][scriptblock]$Mutation
    )

    $path = Join-Path $testDirectory "listing.$Locale.json"
    $listing = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    & $Mutation $listing
    $listing | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $path -Encoding utf8
}

try {
    New-Item -ItemType Directory -Path $testDirectory -ErrorAction Stop | Out-Null
    Copy-Item -LiteralPath (Join-Path $sourceDirectory 'listing.zh-CN.json') -Destination $testDirectory
    Copy-Item -LiteralPath (Join-Path $sourceDirectory 'listing.en-US.json') -Destination $testDirectory

    $valid = & $validator -StoreDirectory $testDirectory | ConvertFrom-Json
    if (-not $valid.validated -or $valid.listing_count -ne 2) {
        throw 'Valid bilingual Store listings did not pass validation.'
    }
    $draftScreenshots = & $screenshotValidator | ConvertFrom-Json
    if ($draftScreenshots.status -cne 'draft' -or $draftScreenshots.screenshots -ne 0 -or $draftScreenshots.complete) {
        throw 'Draft Store screenshot manifest did not pass its explicit incomplete-state validation.'
    }
    Get-ExpectedFailure -Pattern 'not marked complete' -Action { & $screenshotValidator -RequireComplete }

    Write-TestListing -Locale 'en-US' -Mutation { param($listing) $listing.features[0] = 'x' * 201 }
    Get-ExpectedFailure -Pattern 'maximum is 200' -Action {
        & $validator -StoreDirectory $testDirectory
    }

    Copy-Item -LiteralPath (Join-Path $sourceDirectory 'listing.en-US.json') -Destination $testDirectory -Force
    Write-TestListing -Locale 'en-US' -Mutation {
        param($listing)
        $listing.keywords = @('one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight')
    }
    Get-ExpectedFailure -Pattern 'between 1 and 7 keywords' -Action {
        & $validator -StoreDirectory $testDirectory
    }

    Copy-Item -LiteralPath (Join-Path $sourceDirectory 'listing.en-US.json') -Destination $testDirectory -Force
    Write-TestListing -Locale 'en-US' -Mutation { param($listing) $listing.description += ' https://example.invalid' }
    Get-ExpectedFailure -Pattern 'without HTML or URLs' -Action {
        & $validator -StoreDirectory $testDirectory
    }

    [pscustomobject]@{
        schema_version = 1
        valid_listings_accepted = $true
        oversized_feature_rejected = $true
        excess_keywords_rejected = $true
        description_url_rejected = $true
        screenshot_draft_validated = $true
        incomplete_screenshots_rejected_for_release = $true
    } | ConvertTo-Json
}
finally {
    $resolvedTestDirectory = [System.IO.Path]::GetFullPath($testDirectory)
    if (-not $resolvedTestDirectory.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or
        [System.IO.Path]::GetFileName($resolvedTestDirectory) -notlike 'zifile-store-listing-*') {
        throw "Refusing to remove unexpected test directory: $resolvedTestDirectory"
    }
    if (Test-Path -LiteralPath $resolvedTestDirectory) {
        Remove-Item -LiteralPath $resolvedTestDirectory -Recurse -Force
    }
}
