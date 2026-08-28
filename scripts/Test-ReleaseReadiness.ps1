param(
    [string]$RepositoryRoot = (Split-Path $PSScriptRoot -Parent),
    [string]$ReadinessPath,
    [switch]$RequireReleaseReady
)

$ErrorActionPreference = 'Stop'
$root = [System.IO.Path]::GetFullPath($RepositoryRoot)
$defaultReadinessPath = [System.IO.Path]::GetFullPath((Join-Path $root 'release\readiness.json'))
if ([string]::IsNullOrWhiteSpace($ReadinessPath)) {
    $ReadinessPath = $defaultReadinessPath
}
$ReadinessPath = [System.IO.Path]::GetFullPath($ReadinessPath)
$isRepositoryManifest = $ReadinessPath.Equals($defaultReadinessPath, [StringComparison]::OrdinalIgnoreCase)
if (-not (Test-Path -LiteralPath $ReadinessPath -PathType Leaf)) {
    throw "Release readiness manifest is missing: $ReadinessPath"
}

$manifest = Get-Content -Raw -LiteralPath $ReadinessPath | ConvertFrom-Json
if ($manifest.schema_version -ne 1 -or $manifest.product -ne 'ZiFile' -or $manifest.target_release -ne '1.0.0') {
    throw 'Release readiness identity or schema is invalid.'
}
$date = [DateTime]::MinValue
if (-not [DateTime]::TryParseExact([string]$manifest.updated, 'yyyy-MM-dd', [Globalization.CultureInfo]::InvariantCulture, [Globalization.DateTimeStyles]::None, [ref]$date)) {
    throw 'Release readiness updated date must use YYYY-MM-DD.'
}

$required = [ordered]@{
    'public-contract-freeze' = 19
    'foreground-operation-queue' = 11
    'trusted-msix-lifecycle' = 12
    'physical-arm64' = 13
    'accessible-default-ui' = 14
    'formal-store-screenshots' = 15
    'wack-certification' = 16
    'microsoft-store-certification' = 17
    'winget-acceptance' = 18
    'partner-center-identity' = 8
    'production-cloud-hsm-signing' = 19
}
$gates = @($manifest.gates)
if ($gates.Count -ne $required.Count) { throw "Release readiness must contain exactly $($required.Count) gates." }
$seen = @{}
$pending = [Collections.Generic.List[string]]::new()
foreach ($gate in $gates) {
    $id = [string]$gate.id
    if (-not $required.Contains($id) -or $seen.ContainsKey($id)) { throw "Release readiness contains an unknown or duplicate gate: $id" }
    $seen[$id] = $true
    if ([int]$gate.issue -ne $required[$id]) { throw "Release gate $id must track issue #$($required[$id])." }
    if ([string]::IsNullOrWhiteSpace([string]$gate.notes)) { throw "Release gate $id must explain its evidence boundary." }
    $evidence = @($gate.evidence)
    switch ([string]$gate.status) {
        'pending' {
            if ($evidence.Count -ne 0) { throw "Pending release gate $id cannot claim evidence." }
            $pending.Add($id)
        }
        'passed' {
            if ($evidence.Count -eq 0) { throw "Passed release gate $id requires evidence." }
            foreach ($reference in $evidence) {
                if ([string]$reference -notmatch '^https://github\.com/ax2/zifile/(?:actions/runs/\d+|issues/\d+#issuecomment-\d+|pull/\d+(?:#issuecomment-\d+)?|releases/tag/[^/]+)$') {
                    throw "Release gate $id has an unsupported evidence reference: $reference"
                }
            }
        }
        default { throw "Release gate $id has unsupported status: $($gate.status)" }
    }
}
foreach ($id in $required.Keys) { if (-not $seen.ContainsKey($id)) { throw "Release readiness omits gate: $id" } }

$computedStatus = if ($pending.Count -eq 0) { 'ready' } else { 'candidate' }
if ([string]$manifest.overall_status -ne $computedStatus) { throw "Release readiness overall_status must be $computedStatus." }

if ($isRepositoryManifest) {
    $readme = Get-Content -Raw -LiteralPath (Join-Path $root 'README.md')
    $zh = Get-Content -Raw -LiteralPath (Join-Path $root 'docs\src\content\docs\releases\release-readiness.md')
    $en = Get-Content -Raw -LiteralPath (Join-Path $root 'docs\src\content\docs\en\releases\release-readiness.md')
    $workflow = Get-Content -Raw -LiteralPath (Join-Path $root '.github\workflows\release.yml')
    if ($readme -notmatch [Regex]::Escape('(release/readiness.json)')) { throw 'README.md does not link to release/readiness.json.' }
    foreach ($token in @('release/readiness.json', '11', 'RequireReleaseReady')) {
        if ($zh -notmatch [Regex]::Escape($token) -or $en -notmatch [Regex]::Escape($token)) { throw "Localized release-readiness pages omit shared token: $token" }
    }
    $plainZh = $zh -replace '`', ''
    $plainEn = $en -replace '`', ''
    if ($plainZh -notmatch [Regex]::Escape("当前状态是 $computedStatus") -or
        $plainEn -notmatch [Regex]::Escape("current status is $computedStatus")) {
        throw 'Localized release-readiness pages do not match overall_status.'
    }
    if ($workflow -notmatch [Regex]::Escape('Test-ReleaseReadiness.ps1') -or $workflow -notmatch [Regex]::Escape('RequireReleaseReady')) {
        throw 'Release workflow does not enforce release readiness for stable tags.'
    }
}
if ($RequireReleaseReady -and $pending.Count -ne 0) {
    throw "ZiFile 1.0 is not release-ready. Pending gates: $($pending -join ', ')"
}

[pscustomobject]@{
    schema_version = 1
    target_release = [string]$manifest.target_release
    overall_status = $computedStatus
    gates = $gates.Count
    passed = $gates.Count - $pending.Count
    pending = $pending.Count
    pending_ids = @($pending)
    stable_release_allowed = ($pending.Count -eq 0)
} | ConvertTo-Json
