param(
    [string]$ExpectedVersion,
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '..')
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
$changelogPath = Join-Path $repoRoot 'CHANGELOG.md'
$changelog = Get-Content -Raw -LiteralPath $changelogPath

if ($changelog -notmatch '(?m)^# Changelog\s*$') {
    throw 'CHANGELOG.md must start with a Changelog title.'
}
$unreleased = [Regex]::Matches($changelog, '(?m)^## \[Unreleased\]\s*$')
if ($unreleased.Count -ne 1) {
    throw 'CHANGELOG.md must contain exactly one [Unreleased] section.'
}

$result = [ordered]@{
    schema_version = 1
    changelog = 'CHANGELOG.md'
    unreleased_section = $true
    release_version = $null
    release_date = $null
    release_entries = 0
    ready_for_tag = $false
}

if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion)) {
    $versionGate = Join-Path $PSScriptRoot 'Test-VersionConsistency.ps1'
    $version = & $versionGate -RepositoryRoot $repoRoot -ExpectedVersion $ExpectedVersion |
        ConvertFrom-Json
    $escapedVersion = [Regex]::Escape($version.version)
    $heading = [Regex]::Match(
        $changelog,
        "(?m)^## \[$escapedVersion\] - (?<date>\d{4}-\d{2}-\d{2})\s*$"
    )
    if (-not $heading.Success) {
        throw "CHANGELOG.md does not contain release heading '## [$($version.version)] - YYYY-MM-DD'."
    }
    $date = [DateTime]::MinValue
    if (-not [DateTime]::TryParseExact(
        $heading.Groups['date'].Value,
        'yyyy-MM-dd',
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::None,
        [ref]$date
    )) {
        throw "CHANGELOG.md release date is invalid: $($heading.Groups['date'].Value)"
    }
    $bodyStart = $heading.Index + $heading.Length
    $nextHeading = [Regex]::Match($changelog.Substring($bodyStart), '(?m)^## \[')
    $bodyLength = if ($nextHeading.Success) { $nextHeading.Index } else { $changelog.Length - $bodyStart }
    $body = $changelog.Substring($bodyStart, $bodyLength)
    if ($body -notmatch '(?m)^### (Added|Changed|Deprecated|Removed|Fixed|Security)\s*$') {
        throw "CHANGELOG.md release $($version.version) has no Keep a Changelog category."
    }
    $entries = [Regex]::Matches($body, '(?m)^-\s+\S').Count
    if ($entries -eq 0) {
        throw "CHANGELOG.md release $($version.version) has no release entries."
    }
    if ($body -match '(?i)\b(TODO|TBD)\b') {
        throw "CHANGELOG.md release $($version.version) still contains TODO or TBD placeholders."
    }
    $result.release_version = $version.version
    $result.release_date = $date.ToString('yyyy-MM-dd')
    $result.release_entries = $entries
    $result.ready_for_tag = $true
}

$result | ConvertTo-Json
