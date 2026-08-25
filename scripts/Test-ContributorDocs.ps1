param(
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '..')
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
$contributingPath = Join-Path $repoRoot 'CONTRIBUTING.md'
$pullRequestTemplatePath = Join-Path $repoRoot '.github\PULL_REQUEST_TEMPLATE\pull_request_template.md'
$chineseGuidePath = Join-Path $repoRoot 'docs\src\content\docs\development\contributing.md'
$englishGuidePath = Join-Path $repoRoot 'docs\src\content\docs\en\development\contributing.md'
$readmePath = Join-Path $repoRoot 'README.md'

function Assert-Tokens {
    param(
        [string]$Path,
        [string[]]$Tokens
    )

    $source = Get-Content -Raw -LiteralPath $Path
    foreach ($token in $Tokens) {
        if (-not $source.Contains($token, [System.StringComparison]::Ordinal)) {
            throw "$Path does not contain required contributor policy: $token"
        }
    }
}

$commands = @(
    'cargo fmt --all -- --check',
    'cargo clippy --workspace --all-targets --all-features -- -D warnings',
    'cargo test --workspace --all-targets --all-features --locked',
    './tests/smoke/foundation.ps1 -SkipDesktopLaunch',
    './tests/smoke/packaging-policy.ps1',
    'pnpm --dir docs build'
)
$policyTokens = @('CHANGELOG.md', 'SECURITY.md', 'Iced', 'Dioxus', 'ZIP')

Assert-Tokens -Path $contributingPath -Tokens ($commands + $policyTokens)
Assert-Tokens -Path $pullRequestTemplatePath -Tokens ($commands + @('CHANGELOG.md'))
Assert-Tokens -Path $chineseGuidePath -Tokens ($commands + @('CHANGELOG.md', 'SECURITY.md'))
Assert-Tokens -Path $englishGuidePath -Tokens ($commands + @('CHANGELOG.md', 'SECURITY.md'))
Assert-Tokens -Path $readmePath -Tokens @('CONTRIBUTING.md')

[ordered]@{
    schema_version = 1
    commands_checked = $commands.Count
    policy_tokens_checked = $policyTokens.Count
    root_guide = $true
    pull_request_template = $true
    locale_guides = 2
    readme_link = $true
    synchronized = $true
} | ConvertTo-Json
