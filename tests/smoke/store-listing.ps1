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

function Assert-ListingDocumentation {
    param(
        [Parameter(Mandatory)][string]$Locale,
        [Parameter(Mandatory)][string]$DocumentationPath
    )

    $listingPath = Join-Path $sourceDirectory "listing.$Locale.json"
    $listing = Get-Content -Raw -LiteralPath $listingPath | ConvertFrom-Json
    $documentation = Get-Content -Raw -LiteralPath $DocumentationPath
    $requiredCopy = @($listing.short_description) +
        @(([string]$listing.description -split "`r?`n") | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) +
        @($listing.features)
    foreach ($text in $requiredCopy) {
        if (-not $documentation.Contains([string]$text, [StringComparison]::Ordinal)) {
            throw "$Locale Store documentation is out of sync with structured listing copy: $text"
        }
    }
}

function Write-TestScreenshot {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][int]$Marker,
        [int]$Width = 1366,
        [int]$Height = 768
    )
    $directory = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
    $bitmap = [Drawing.Bitmap]::new($Width, $Height)
    try {
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try { $graphics.Clear([Drawing.Color]::FromArgb(255, 28 + $Marker, 38, 52)) }
        finally { $graphics.Dispose() }
        $bitmap.SetPixel($Marker, $Marker, [Drawing.Color]::FromArgb(255, 80, 180, 255))
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $bitmap.Dispose() }
}

try {
    New-Item -ItemType Directory -Path $testDirectory -ErrorAction Stop | Out-Null
    Copy-Item -LiteralPath (Join-Path $sourceDirectory 'listing.zh-CN.json') -Destination $testDirectory
    Copy-Item -LiteralPath (Join-Path $sourceDirectory 'listing.en-US.json') -Destination $testDirectory

    $valid = & $validator -StoreDirectory $testDirectory | ConvertFrom-Json
    if (-not $valid.validated -or $valid.listing_count -ne 2) {
        throw 'Valid bilingual Store listings did not pass validation.'
    }
    Assert-ListingDocumentation `
        -Locale 'zh-CN' `
        -DocumentationPath (Join-Path $repoRoot 'docs\src\content\docs\releases\store-listing.md')
    Assert-ListingDocumentation `
        -Locale 'en-US' `
        -DocumentationPath (Join-Path $repoRoot 'docs\src\content\docs\en\releases\store-listing.md')
    $draftScreenshots = & $screenshotValidator | ConvertFrom-Json
    if ($draftScreenshots.status -cne 'draft' -or $draftScreenshots.screenshots -ne 0 -or $draftScreenshots.complete) {
        throw 'Draft Store screenshot manifest did not pass its explicit incomplete-state validation.'
    }
    Get-ExpectedFailure -Pattern 'not marked complete' -Action { & $screenshotValidator -RequireComplete }

    Add-Type -AssemblyName System.Drawing
    $scenarios = @('home', 'create', 'browse', 'extract')
    $localeSets = @()
    $marker = 0
    foreach ($locale in @('zh-CN', 'en-US')) {
        $shots = @()
        for ($index = 0; $index -lt $scenarios.Count; $index++) {
            $marker++
            $relative = "assets/$locale/desktop/$($index + 1)-$($scenarios[$index]).png"
            $absolute = Join-Path $testDirectory ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
            Write-TestScreenshot -Path $absolute -Marker $marker
            $shots += [ordered]@{
                order = $index + 1
                scenario = $scenarios[$index]
                path = $relative
                caption = "$locale $($scenarios[$index])"
                sha256 = (Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash
            }
        }
        $localeSets += [ordered]@{ locale = $locale; screenshots = $shots }
    }
    $screenshotManifestPath = Join-Path $testDirectory 'screenshots.json'
    $screenshotManifest = [ordered]@{
        schema_version = 1
        status = 'complete'
        source_commit = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
        requirements_source = 'https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/screenshots-and-images'
        locales = $localeSets
    }
    $screenshotManifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $screenshotManifestPath -Encoding utf8
    $completeScreenshots = & $screenshotValidator -ManifestPath $screenshotManifestPath -RequireComplete | ConvertFrom-Json
    if (-not $completeScreenshots.complete -or $completeScreenshots.screenshots -ne 8) {
        throw 'A complete bilingual eight-screenshot manifest did not pass validation.'
    }

    $first = $screenshotManifest.locales[0].screenshots[0]
    $firstPath = Join-Path $testDirectory ($first.path -replace '/', [IO.Path]::DirectorySeparatorChar)
    Write-TestScreenshot -Path $firstPath -Marker 20 -Width 800 -Height 600
    $first.sha256 = (Get-FileHash -LiteralPath $firstPath -Algorithm SHA256).Hash
    $screenshotManifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $screenshotManifestPath -Encoding utf8
    Get-ExpectedFailure -Pattern 'smaller than 1366x768' -Action {
        & $screenshotValidator -ManifestPath $screenshotManifestPath -RequireComplete
    }

    Write-TestScreenshot -Path $firstPath -Marker 21
    $first.sha256 = (Get-FileHash -LiteralPath $firstPath -Algorithm SHA256).Hash
    $second = $screenshotManifest.locales[0].screenshots[1]
    $secondPath = Join-Path $testDirectory ($second.path -replace '/', [IO.Path]::DirectorySeparatorChar)
    Copy-Item -LiteralPath $firstPath -Destination $secondPath -Force
    $second.sha256 = $first.sha256
    $screenshotManifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $screenshotManifestPath -Encoding utf8
    Get-ExpectedFailure -Pattern 'Duplicate screenshot content' -Action {
        & $screenshotValidator -ManifestPath $screenshotManifestPath -RequireComplete
    }

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
        listing_documentation_synchronized = $true
        oversized_feature_rejected = $true
        excess_keywords_rejected = $true
        description_url_rejected = $true
        screenshot_draft_validated = $true
        incomplete_screenshots_rejected_for_release = $true
        complete_bilingual_screenshots_accepted = $true
        undersized_screenshot_rejected = $true
        duplicate_screenshot_rejected = $true
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
