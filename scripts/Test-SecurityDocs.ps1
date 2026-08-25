param(
    [string]$RepositoryRoot = (Split-Path $PSScriptRoot -Parent)
)

$ErrorActionPreference = 'Stop'

$root = [System.IO.Path]::GetFullPath($RepositoryRoot)
$policyPath = Join-Path $root 'SECURITY.md'
$readmePath = Join-Path $root 'README.md'
$pages = @(
    (Join-Path $root 'docs\src\content\docs\architecture\security.md'),
    (Join-Path $root 'docs\src\content\docs\en\architecture\security.md')
)

foreach ($path in @($policyPath) + $pages) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required security document is missing: $path"
    }
}
if (-not (Test-Path -LiteralPath $readmePath -PathType Leaf) -or
    (Get-Content -Raw -LiteralPath $readmePath) -notmatch [Regex]::Escape('(SECURITY.md)')) {
    throw 'README.md does not link to the repository security policy.'
}

$policy = Get-Content -Raw -LiteralPath $policyPath
$normalizedPolicy = $policy -replace '\s+', ' '
foreach ($required in @(
    'GitHub private vulnerability reporting is not currently enabled',
    'ax2@zicode.com',
    'ZiFile security report',
    'public issue containing an unpatched vulnerability',
    'does not currently promise a fixed response SLA',
    'only the current default branch receives security fixes',
    'working exploit',
    'private customer data'
)) {
    if ($normalizedPolicy -notmatch [Regex]::Escape($required)) {
        throw "SECURITY.md omits required policy text: $required"
    }
}

$zh = Get-Content -Raw -LiteralPath $pages[0]
$en = Get-Content -Raw -LiteralPath $pages[1]
foreach ($shared in @('ax2@zicode.com', 'ZiFile security report', 'SECURITY.md')) {
    if ($zh -notmatch [Regex]::Escape($shared) -or $en -notmatch [Regex]::Escape($shared)) {
        throw "Localized security pages are missing shared token: $shared"
    }
}
if ($zh -notmatch '尚未启用 GitHub 私密漏洞报告' -or
    $en -notmatch 'private vulnerability reporting is not currently enabled') {
    throw 'Localized security pages do not state the current private-reporting status.'
}
if ($zh -notmatch '公开 Issue' -or $en -notmatch 'public issue') {
    throw 'Localized security pages do not prohibit public vulnerability details.'
}

[pscustomobject]@{
    schema_version = 1
    synchronized = $true
    locale_pages = $pages.Count
    private_reporting_enabled = $false
    fallback_contact = 'ax2@zicode.com'
} | ConvertTo-Json
