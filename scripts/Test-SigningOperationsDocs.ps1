param(
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '..')
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath($RepositoryRoot)
$chinesePath = Join-Path $repoRoot 'docs\src\content\docs\development\signing-operations.md'
$englishPath = Join-Path $repoRoot 'docs\src\content\docs\en\development\signing-operations.md'
$releaseWorkflowPath = Join-Path $repoRoot '.github\workflows\release.yml'

foreach ($requiredPath in @($chinesePath, $englishPath, $releaseWorkflowPath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Signing operations policy file is missing: $requiredPath"
    }
}

function Assert-Tokens {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string[]]$Tokens
    )

    $source = Get-Content -Raw -LiteralPath $Path
    foreach ($token in $Tokens) {
        if (-not $source.Contains($token, [StringComparison]::Ordinal)) {
            throw "$Path omits required signing-operations token: $token"
        }
    }
}

$sharedTokens = @(
    'production-signing',
    'signing_provider=digicert-stm',
    'ZIFILE_MSIX_IDENTITY',
    'ZIFILE_MSIX_PUBLISHER',
    'SM_HOST',
    'SM_KEYPAIR_ALIAS',
    'SM_API_KEY',
    'SM_CLIENT_CERT_FILE_B64',
    'SM_CLIENT_CERT_PASSWORD',
    'signed-windows-x64',
    'signed-windows-arm64',
    'release/readiness.json',
    'SHA256SUMS-',
    'WACK'
)
Assert-Tokens -Path $chinesePath -Tokens ($sharedTokens + @('应急停止', '吊销', '轮换', '代码签名私钥'))
Assert-Tokens -Path $englishPath -Tokens ($sharedTokens + @('Emergency stop', 'revocation', 'Rotation', 'code-signing private key'))

Assert-Tokens -Path $releaseWorkflowPath -Tokens @(
    'permissions:',
    'contents: read',
    'environment: production-signing',
    'timeout-minutes: 30',
    'production-signing-${{ matrix.architecture }}',
    'cancel-in-progress: false',
    'Test-CloudSigningInputs.ps1',
    'digicert/code-signing-software-trust-action@v1.2.1',
    'Test-SignedReleaseArtifacts.ps1',
    'Remove ephemeral client authentication certificate',
    'signed-windows-${{ matrix.architecture }}'
)

[ordered]@{
    schema_version = 1
    locale_pages = 2
    shared_tokens = $sharedTokens.Count
    credential_classes = 7
    rotation_runbook = $true
    emergency_stop_runbook = $true
    revocation_runbook = $true
    least_privilege_workflow = $true
    signing_timeout = $true
    signing_concurrency = $true
    synchronized = $true
} | ConvertTo-Json
