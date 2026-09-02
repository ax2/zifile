param(
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '..')
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath($RepositoryRoot)

function Read-RequiredFile {
    param([Parameter(Mandatory)][string]$RelativePath)

    $path = Join-Path $repoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "SignPath readiness file is missing: $RelativePath"
    }
    return Get-Content -Raw -LiteralPath $path
}

function Assert-Contains {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string[]]$Tokens
    )

    foreach ($token in $Tokens) {
        if (-not $Source.Contains($token, [StringComparison]::Ordinal)) {
            throw "$Path omits required SignPath readiness text: $token"
        }
    }
}

$readme = Read-RequiredFile 'README.md'
$policy = Read-RequiredFile 'CODE-SIGNING-POLICY.md'
$englishPolicy = Read-RequiredFile 'docs/src/content/docs/en/development/code-signing-policy.md'
$chinesePolicy = Read-RequiredFile 'docs/src/content/docs/development/code-signing-policy.md'
$englishReleasing = Read-RequiredFile 'docs/src/content/docs/en/development/releasing.md'
$chineseReleasing = Read-RequiredFile 'docs/src/content/docs/development/releasing.md'
$msixReadme = Read-RequiredFile 'packaging/msix/README.md'
$wingetReadme = Read-RequiredFile 'packaging/winget/README.md'
$codeowners = Read-RequiredFile '.github/CODEOWNERS'
$license = Read-RequiredFile 'LICENSE'

$attribution = 'Free code signing provided by SignPath.io, certificate by SignPath Foundation.'
$commonPolicyTokens = @(
    'Code signing policy',
    $attribution,
    'https://ax2.github.io/zifile/en/product/privacy/',
    'https://ax2.github.io/zifile/product/privacy/',
    '@ax2',
    'multi-factor authentication',
    'private keys',
    'Identity Publisher',
    'GitHub Actions'
)

Assert-Contains -Path 'README.md' -Source $readme -Tokens @(
    'Code signing policy',
    'CODE-SIGNING-POLICY.md',
    'https://github.com/ax2/zifile'
)
Assert-Contains -Path 'CODE-SIGNING-POLICY.md' -Source $policy -Tokens $commonPolicyTokens
Assert-Contains -Path 'docs/src/content/docs/en/development/code-signing-policy.md' -Source $englishPolicy -Tokens @(
    'Code signing policy',
    $attribution,
    'https://ax2.github.io/zifile/en/product/privacy/',
    '@ax2',
    'Identity Publisher'
)
Assert-Contains -Path 'docs/src/content/docs/development/code-signing-policy.md' -Source $chinesePolicy -Tokens @(
    '代码签名政策',
    $attribution,
    'https://github.com/ax2/zifile/blob/main/CODE-SIGNING-POLICY.md',
    '@ax2',
    'Identity Publisher'
)
Assert-Contains -Path 'packaging/msix/README.md' -Source $msixReadme -Tokens @(
    'Code signing policy',
    'CODE-SIGNING-POLICY.md'
)
Assert-Contains -Path 'packaging/winget/README.md' -Source $wingetReadme -Tokens @(
    'Code signing policy',
    'CODE-SIGNING-POLICY.md'
)
Assert-Contains -Path 'docs/src/content/docs/en/development/releasing.md' -Source $englishReleasing -Tokens @(
    '/zifile/en/development/code-signing-policy/'
)
Assert-Contains -Path 'docs/src/content/docs/development/releasing.md' -Source $chineseReleasing -Tokens @(
    '/zifile/development/code-signing-policy/'
)
Assert-Contains -Path '.github/CODEOWNERS' -Source $codeowners -Tokens @('* @ax2')
Assert-Contains -Path 'LICENSE' -Source $license -Tokens @('MIT License')

[ordered]@{
    schema_version = 1
    canonical_policy = $true
    localized_policy_pages = 2
    release_documentation_links_policy = $true
    roles_documented = $true
    privacy_links_documented = $true
    mfa_requirement_documented = $true
    msix_identity_boundary_documented = $true
    mit_license_detected = $true
    application_status = 'preparation'
    certificate_approval = 'pending'
} | ConvertTo-Json
