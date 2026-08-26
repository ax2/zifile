param(
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '..')
)

$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($RepositoryRoot)
$paths = @(
    'docs/src/content/docs/guides/getting-started.md',
    'docs/src/content/docs/guides/troubleshooting.md',
    'docs/src/content/docs/en/guides/getting-started.md',
    'docs/src/content/docs/en/guides/troubleshooting.md'
)
$sources = @{}
foreach ($relativePath in $paths) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing user guide: $relativePath"
    }
    $sources[$relativePath] = Get-Content -Raw -LiteralPath $path
}

$sharedTokens = @(
    'Ctrl+O',
    'Ctrl+N',
    'Escape',
    '500',
    '32',
    '--password-stdin',
    'RAR',
    'CAB',
    'Worker',
    'Microsoft Store',
    'WinGet',
    'GitHub Issues'
)
foreach ($token in $sharedTokens) {
    $localeSources = @{
        'zh-CN' = @(
            $sources['docs/src/content/docs/guides/getting-started.md'],
            $sources['docs/src/content/docs/guides/troubleshooting.md']
        ) -join "`n"
        'en' = @(
            $sources['docs/src/content/docs/en/guides/getting-started.md'],
            $sources['docs/src/content/docs/en/guides/troubleshooting.md']
        ) -join "`n"
    }
    foreach ($locale in $localeSources.Keys) {
        $localeSource = $localeSources[$locale]
        if ($localeSource -notmatch [Regex]::Escape($token)) {
            throw "User guides for '$locale' omit required capability or boundary token: $token"
        }
    }
}

$config = Get-Content -Raw -LiteralPath (Join-Path $root 'docs/astro.config.mjs')
$zhIndex = Get-Content -Raw -LiteralPath (Join-Path $root 'docs/src/content/docs/index.md')
$enIndex = Get-Content -Raw -LiteralPath (Join-Path $root 'docs/src/content/docs/en/index.md')
$readme = Get-Content -Raw -LiteralPath (Join-Path $root 'README.md')
foreach ($requiredToken in @(
    "directory: 'guides'",
    "translations: { en: 'User guides' }"
)) {
    if ($config -notmatch [Regex]::Escape($requiredToken)) {
        throw "Documentation navigation omits user-guide token: $requiredToken"
    }
}
if ($zhIndex -notmatch [Regex]::Escape('/zifile/guides/getting-started/') -or
    $enIndex -notmatch [Regex]::Escape('/zifile/en/guides/getting-started/') -or
    $readme -notmatch [Regex]::Escape('en/guides/getting-started.md') -or
    $readme -notmatch [Regex]::Escape('en/guides/troubleshooting.md')) {
    throw 'The documentation home pages or repository README do not expose both user guides.'
}

[pscustomobject]@{
    schema_version = 1
    synchronized = $true
    locale_pairs = 2
    guide_pages = $paths.Count
    shared_tokens = $sharedTokens.Count
    navigation_wired = $true
    alpha_distribution_boundary_documented = $true
    password_cli_boundary_documented = $true
    safe_reporting_documented = $true
} | ConvertTo-Json
