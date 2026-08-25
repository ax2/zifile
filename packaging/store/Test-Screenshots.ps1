param(
    [string]$ManifestPath = (Join-Path $PSScriptRoot 'screenshots.json'),
    [switch]$RequireComplete
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$manifestFile = [IO.Path]::GetFullPath($ManifestPath)
$storeRoot = [IO.Path]::GetFullPath((Split-Path -Parent $manifestFile))
$manifest = Get-Content -Raw -LiteralPath $manifestFile | ConvertFrom-Json

function Get-BigEndianUInt32 {
    param([byte[]]$Bytes, [int]$Offset)
    return ([uint32]$Bytes[$Offset] -shl 24) -bor
        ([uint32]$Bytes[$Offset + 1] -shl 16) -bor
        ([uint32]$Bytes[$Offset + 2] -shl 8) -bor
        [uint32]$Bytes[$Offset + 3]
}

function Get-PngDimensions {
    param([Parameter(Mandatory)][string]$Path)
    $stream = [IO.File]::OpenRead($Path)
    try {
        if ($stream.Length -lt 24) { throw "PNG is too short: $Path" }
        $header = [byte[]]::new(24)
        if ($stream.Read($header, 0, 24) -ne 24) { throw "Could not read PNG header: $Path" }
    }
    finally { $stream.Dispose() }
    $signature = [byte[]](137, 80, 78, 71, 13, 10, 26, 10)
    for ($index = 0; $index -lt $signature.Length; $index++) {
        if ($header[$index] -ne $signature[$index]) { throw "Invalid PNG signature: $Path" }
    }
    if ((Get-BigEndianUInt32 $header 8) -ne 13 -or
        [Text.Encoding]::ASCII.GetString($header, 12, 4) -cne 'IHDR') {
        throw "PNG does not begin with a valid IHDR chunk: $Path"
    }
    [pscustomobject]@{ width = Get-BigEndianUInt32 $header 16; height = Get-BigEndianUInt32 $header 20 }
}

if ($manifest.schema_version -ne 1) { throw 'Screenshot manifest schema_version must be 1.' }
if ($manifest.status -notin @('draft', 'complete')) { throw 'Screenshot manifest status must be draft or complete.' }
if ($RequireComplete -and $manifest.status -cne 'complete') { throw 'Store screenshots are not marked complete.' }
if ($manifest.status -ceq 'complete' -and $manifest.source_commit -notmatch '^[0-9a-f]{40}$') {
    throw 'Complete Store screenshots require a 40-character source_commit.'
}
if ($manifest.status -ceq 'complete') {
    if ($null -eq $manifest.capture) { throw 'Complete Store screenshots require capture metadata.' }
    if ($manifest.capture.app_version -notmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') {
        throw 'Screenshot capture metadata requires a semantic app_version.'
    }
    if ($manifest.capture.windows_build -notmatch '^\d+\.\d+\.\d+$') {
        throw 'Screenshot capture metadata requires a three-part Windows build.'
    }
    if ($manifest.capture.theme -notin @('light', 'dark')) { throw 'Screenshot capture theme must be light or dark.' }
    if ($manifest.capture.scale_percent -lt 100 -or $manifest.capture.scale_percent -gt 500) {
        throw 'Screenshot capture scale_percent must be between 100 and 500.'
    }
    $capturedValue = $manifest.capture.captured_at_utc
    $captured = [DateTimeOffset]::MinValue
    $validCapturedAt = if ($capturedValue -is [DateTime]) {
        $capturedValue.Kind -eq [DateTimeKind]::Utc
    }
    else {
        [DateTimeOffset]::TryParseExact(
            [string]$capturedValue,
            "yyyy-MM-dd'T'HH:mm:ss'Z'",
            [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::AssumeUniversal,
            [ref]$captured)
    }
    if (-not $validCapturedAt) {
        throw 'Screenshot capture captured_at_utc must use yyyy-MM-ddTHH:mm:ssZ.'
    }
    if ($manifest.capture.candidate_kind -notin @('signed-msix', 'store-signed-msix')) {
        throw 'Screenshot capture candidate_kind must identify a signed candidate.'
    }
}
if ($manifest.requirements_source -cne 'https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/screenshots-and-images') {
    throw 'Screenshot manifest requirements_source must reference the pinned Microsoft Store guidance page.'
}

$expectedLocales = @('zh-CN', 'en-US')
$seenHashes = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$validated = 0
foreach ($locale in $expectedLocales) {
    $sets = @($manifest.locales | Where-Object locale -CEQ $locale)
    if ($sets.Count -ne 1) { throw "Screenshot manifest must contain locale exactly once: $locale" }
    $screenshots = @($sets[0].screenshots)
    if ($screenshots.Count -gt 10) { throw "$locale exceeds the 10 Desktop screenshot maximum." }
    if (($RequireComplete -or $manifest.status -ceq 'complete') -and $screenshots.Count -lt 4) {
        throw "$locale requires at least four completed Desktop screenshots."
    }
    if ($RequireComplete -or $manifest.status -ceq 'complete') {
        $requiredScenarios = @('browse', 'create', 'extract', 'home')
        $actualScenarios = @($screenshots | ForEach-Object scenario | Sort-Object -Unique)
        if (($actualScenarios -join ',') -cne ($requiredScenarios -join ',')) {
            throw "$locale must cover home, create, browse, and extract screenshot scenarios."
        }
    }
    $orders = @($screenshots | ForEach-Object order | Sort-Object)
    if (($orders -join ',') -cne ((1..$screenshots.Count) -join ',')) {
        if ($screenshots.Count -ne 0) { throw "$locale screenshot order must be contiguous from 1." }
    }
    foreach ($shot in $screenshots) {
        if ($shot.caption.Length -lt 1 -or $shot.caption.Length -gt 200) { throw "$locale screenshot caption must contain 1-200 characters." }
        if ($shot.scenario -notin @('home', 'create', 'browse', 'extract')) { throw "$locale has an unsupported screenshot scenario: $($shot.scenario)" }
        if ($shot.sha256 -notmatch '^[0-9A-Fa-f]{64}$') { throw "$locale screenshot requires a SHA-256 value." }
        $relative = [string]$shot.path
        $requiredPrefix = "assets/$locale/desktop/"
        if (-not $relative.StartsWith($requiredPrefix, [StringComparison]::Ordinal) -or [IO.Path]::GetExtension($relative) -cne '.png') {
            throw "$locale screenshot must be a PNG below $requiredPrefix"
        }
        $resolved = [IO.Path]::GetFullPath((Join-Path $storeRoot ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)))
        if (-not $resolved.StartsWith($storeRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Screenshot path escapes the Store directory.' }
        $file = Get-Item -LiteralPath $resolved
        if ($file.Length -gt 50MB) { throw "Screenshot exceeds 50 MB: $relative" }
        $dimensions = Get-PngDimensions $resolved
        $validLandscape = $dimensions.width -ge 1366 -and $dimensions.height -ge 768
        $validPortrait = $dimensions.width -ge 768 -and $dimensions.height -ge 1366
        if (-not ($validLandscape -or $validPortrait)) { throw "Desktop screenshot is smaller than 1366x768 (or portrait equivalent): $relative" }
        $actualHash = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash
        if ($actualHash -cne $shot.sha256.ToUpperInvariant()) { throw "Screenshot hash mismatch: $relative" }
        if (-not $seenHashes.Add($actualHash)) { throw "Duplicate screenshot content is not allowed: $relative" }
        $validated++
    }
}

[pscustomobject]@{ schema_version = 1; status = $manifest.status; locales = 2; screenshots = $validated; complete = $manifest.status -ceq 'complete' } | ConvertTo-Json
