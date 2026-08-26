$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$publishingPolicy = Join-Path $repoRoot 'packaging\msix\Test-PublishingInputs.ps1'
$packageAudit = Join-Path $repoRoot 'packaging\msix\Test-Package.ps1'
$packageBuild = Join-Path $repoRoot 'packaging\msix\Build-Package.ps1'
$packageLifecycle = Join-Path $repoRoot 'tests\smoke\msix-lifecycle.ps1'
$repairHelper = Join-Path $repoRoot 'tests\helpers\msix-repair\Program.cs'
$repairProject = Join-Path $repoRoot 'tests\helpers\msix-repair\MsixRepair.csproj'
$repairProbe = Join-Path $repoRoot 'tests\helpers\msix-repair\Invoke-Probe.ps1'
$rarCorpus = Join-Path $repoRoot 'tests\interoperability\rar-corpus.ps1'
$wackReadiness = Join-Path $repoRoot 'packaging\msix\Test-WackReadiness.ps1'
$versionConsistency = Join-Path $repoRoot 'scripts\Test-VersionConsistency.ps1'
$releaseNotes = Join-Path $repoRoot 'scripts\Test-ReleaseNotes.ps1'
$contributorDocs = Join-Path $repoRoot 'scripts\Test-ContributorDocs.ps1'
$securityDocs = Join-Path $repoRoot 'scripts\Test-SecurityDocs.ps1'
$releaseReadiness = Join-Path $repoRoot 'scripts\Test-ReleaseReadiness.ps1'

$scriptsToParse = @($publishingPolicy, $packageAudit, $packageBuild, $packageLifecycle, $repairProbe, $rarCorpus, $wackReadiness, $versionConsistency, $releaseNotes, $contributorDocs, $securityDocs, $releaseReadiness)
foreach ($script in $scriptsToParse) {
    $tokens = $null
    $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile(
        $script,
        [ref]$tokens,
        [ref]$errors
    ) | Out-Null
    if ($errors.Count -gt 0) {
        throw "PowerShell parser rejected $(Split-Path $script -Leaf): $($errors -join '; ')"
    }
}

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
        return $_.Exception.Message
    }
    throw "Expected action to fail with '$Pattern'."
}

$versionResult = & $versionConsistency | ConvertFrom-Json
if (-not $versionResult.consistent -or $versionResult.version -ne $versionResult.docs_version) {
    throw 'The workspace version consistency gate rejected the current release version.'
}
if ($versionResult.version -notmatch '^(\d+)\.(\d+)\.(\d+)(?:-(?:alpha|beta|rc)\.(\d+))?$') {
    throw 'The current version did not satisfy the tested release version grammar.'
}
$expectedMsix = "$($Matches[1]).$($Matches[2]).$($Matches[3]).$(if ($Matches[4]) { $Matches[4] } else { '0' })"
if ($versionResult.tag -ne "v$($versionResult.version)" -or $versionResult.msix_version -ne $expectedMsix) {
    throw 'Semantic version conversion did not produce the expected tag and MSIX version.'
}
$acceptedVersion = & $versionConsistency -ExpectedVersion $versionResult.tag | ConvertFrom-Json
if (-not $acceptedVersion.consistent) {
    throw 'The exact workspace release tag was not accepted.'
}
$null = Get-ExpectedFailure -Pattern 'does not match workspace version' -Action {
    & $versionConsistency -ExpectedVersion "$($versionResult.tag)-mismatch"
}
$temporaryBase = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$versionFixture = [System.IO.Path]::GetFullPath((Join-Path $temporaryBase (
    'zifile-version-policy-{0}' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $versionFixture.StartsWith($temporaryBase, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the version-policy fixture outside the system temporary directory.'
}
try {
    New-Item -ItemType Directory -Path (Join-Path $versionFixture 'docs') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Destination $versionFixture
    Copy-Item -LiteralPath (Join-Path $repoRoot 'Cargo.lock') -Destination $versionFixture
    $fixturePackage = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'docs\package.json') |
        ConvertFrom-Json
    $fixturePackage.version = '9.9.9'
    $fixturePackage | ConvertTo-Json -Depth 10 |
        Set-Content -LiteralPath (Join-Path $versionFixture 'docs\package.json') -Encoding utf8NoBOM
    $null = Get-ExpectedFailure -Pattern 'docs/package.json version' -Action {
        & $versionConsistency -RepositoryRoot $versionFixture
    }
}
finally {
    if (Test-Path -LiteralPath $versionFixture) {
        Remove-Item -LiteralPath $versionFixture -Recurse -Force
    }
}

$unreleasedNotes = & $releaseNotes | ConvertFrom-Json
if (-not $unreleasedNotes.unreleased_section -or $unreleasedNotes.ready_for_tag) {
    throw 'The changelog structure gate rejected the current Unreleased state.'
}
$contributorResult = & $contributorDocs | ConvertFrom-Json
if (-not $contributorResult.synchronized -or $contributorResult.locale_guides -ne 2) {
    throw 'Contributor documentation is not synchronized with repository policy.'
}
$securityResult = & $securityDocs | ConvertFrom-Json
if (-not $securityResult.synchronized -or $securityResult.locale_pages -ne 2) {
    throw 'Security documentation is not synchronized with repository policy.'
}
$readinessResult = & $releaseReadiness | ConvertFrom-Json
if ($readinessResult.gates -ne 11 -or
    ($readinessResult.passed + $readinessResult.pending) -ne 11 -or
    $readinessResult.stable_release_allowed -ne ($readinessResult.pending -eq 0)) {
    throw 'The current 1.0 release readiness boundary is invalid.'
}
$readinessFixture = [System.IO.Path]::GetFullPath((Join-Path $temporaryBase (
    'zifile-release-readiness-{0}.json' -f [Guid]::NewGuid().ToString('N')
)))
$pendingReadinessFixture = [System.IO.Path]::GetFullPath((Join-Path $temporaryBase (
    'zifile-release-readiness-pending-{0}.json' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $readinessFixture.StartsWith($temporaryBase, [System.StringComparison]::OrdinalIgnoreCase) -or
    -not $pendingReadinessFixture.StartsWith($temporaryBase, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the release-readiness fixture outside the system temporary directory.'
}
try {
    $readyManifest = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'release\readiness.json') |
        ConvertFrom-Json
    $readyManifest.overall_status = 'ready'
    foreach ($gate in $readyManifest.gates) {
        $gate.status = 'passed'
        $gate.evidence = @("https://github.com/ax2/zifile/issues/$($gate.issue)#issuecomment-1")
    }
    $readyManifest | ConvertTo-Json -Depth 10 |
        Set-Content -LiteralPath $readinessFixture -Encoding utf8NoBOM
    $acceptedReadiness = & $releaseReadiness -ReadinessPath $readinessFixture -RequireReleaseReady |
        ConvertFrom-Json
    if (-not $acceptedReadiness.stable_release_allowed -or $acceptedReadiness.passed -ne 11) {
        throw 'A complete evidenced release-readiness manifest was not accepted.'
    }
    $readyManifest.overall_status = 'candidate'
    $readyManifest.gates[0].status = 'pending'
    $readyManifest.gates[0].evidence = @()
    $readyManifest | ConvertTo-Json -Depth 10 |
        Set-Content -LiteralPath $pendingReadinessFixture -Encoding utf8NoBOM
    $null = Get-ExpectedFailure -Pattern 'not release-ready' -Action {
        & $releaseReadiness -ReadinessPath $pendingReadinessFixture -RequireReleaseReady
    }
}
finally {
    foreach ($fixture in @($readinessFixture, $pendingReadinessFixture)) {
        if (Test-Path -LiteralPath $fixture) {
            Remove-Item -LiteralPath $fixture -Force
        }
    }
}
$null = Get-ExpectedFailure -Pattern 'does not contain release heading' -Action {
    & $releaseNotes -ExpectedVersion $versionResult.tag
}
$releaseFixture = [System.IO.Path]::GetFullPath((Join-Path $temporaryBase (
    'zifile-release-notes-{0}' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $releaseFixture.StartsWith($temporaryBase, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the release-notes fixture outside the system temporary directory.'
}
try {
    New-Item -ItemType Directory -Path (Join-Path $releaseFixture 'docs') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Destination $releaseFixture
    Copy-Item -LiteralPath (Join-Path $repoRoot 'Cargo.lock') -Destination $releaseFixture
    Copy-Item -LiteralPath (Join-Path $repoRoot 'docs\package.json') -Destination (Join-Path $releaseFixture 'docs')
    @"
# Changelog

## [Unreleased]

## [$($versionResult.version)] - 2026-08-26

### Added

- Candidate release evidence.
"@ | Set-Content -LiteralPath (Join-Path $releaseFixture 'CHANGELOG.md') -Encoding utf8NoBOM
    $taggedNotes = & $releaseNotes -RepositoryRoot $releaseFixture -ExpectedVersion $versionResult.tag |
        ConvertFrom-Json
    if (-not $taggedNotes.ready_for_tag -or $taggedNotes.release_entries -ne 1) {
        throw 'A complete versioned changelog section was not accepted.'
    }
    Add-Content -LiteralPath (Join-Path $releaseFixture 'CHANGELOG.md') -Value '- TODO: placeholder'
    $null = Get-ExpectedFailure -Pattern 'TODO or TBD' -Action {
        & $releaseNotes -RepositoryRoot $releaseFixture -ExpectedVersion $versionResult.tag
    }
}
finally {
    if (Test-Path -LiteralPath $releaseFixture) {
        Remove-Item -LiteralPath $releaseFixture -Recurse -Force
    }
}

$missingFailure = Get-ExpectedFailure -Pattern 'ZIFILE_MSIX_IDENTITY' -Action {
    & $publishingPolicy -IdentityName '' -Publisher ''
}
foreach ($secretName in @(
    'ZIFILE_MSIX_IDENTITY',
    'ZIFILE_MSIX_PUBLISHER',
    'ZIFILE_PFX_BASE64',
    'ZIFILE_PFX_PASSWORD'
)) {
    if ($missingFailure -notmatch [Regex]::Escape($secretName)) {
        throw "Missing publishing input diagnostic omitted $secretName."
    }
}

$null = Get-ExpectedFailure -Pattern 'development MSIX identity' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile.Dev' `
        -Publisher 'CN=ZiCode Official' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}
$null = Get-ExpectedFailure -Pattern 'unsigned development publisher' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Development, OID.2.25.311729368913984317654407730594956997722=1' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}
$accepted = & $publishingPolicy `
    -IdentityName 'ZiCode.ZiFile' `
    -Publisher 'CN=ZiCode Official' `
    -ReleaseVersion 'v0.1.0-alpha.1' `
    -SigningCertificateAvailable `
    -SigningPasswordAvailable |
    ConvertFrom-Json
if (-not $accepted.validated) {
    throw 'Formal publishing inputs were not accepted.'
}
if ($accepted.signing_provider -ne 'pfx-scaffolding') {
    throw 'Pre-release publishing policy did not identify the PFX path as scaffolding.'
}
$null = Get-ExpectedFailure -Pattern 'cloud-HSM signing integration' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Official' `
        -ReleaseVersion 'v1.0.0' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}
$null = Get-ExpectedFailure -Pattern 'supported semantic version' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Official' `
        -ReleaseVersion 'release' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}

$buildSource = Get-Content -Raw -LiteralPath $packageBuild
if ($buildSource -notmatch [Regex]::Escape("Test-Package.ps1")) {
    throw 'Build-Package.ps1 does not invoke the package auditor.'
}
$releaseSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\release.yml')
$lifecycleWorkflowSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\msix-lifecycle.yml')
if ($releaseSource -notmatch [Regex]::Escape('Test-PublishingInputs.ps1')) {
    throw 'The release workflow does not invoke the publishing input policy.'
}
if ($releaseSource -notmatch [Regex]::Escape('Test-VersionConsistency.ps1 @arguments')) {
    throw 'The release workflow does not enforce the workspace version source.'
}
if ($releaseSource -notmatch [Regex]::Escape('Test-ReleaseNotes.ps1 @arguments')) {
    throw 'The release workflow does not enforce versioned release notes.'
}
if ($releaseSource -notmatch [Regex]::Escape('Test-ReleaseReadiness.ps1 @readinessArguments') -or
    $releaseSource -notmatch [Regex]::Escape('$readinessArguments.RequireReleaseReady = $true')) {
    throw 'The release workflow does not reject unresolved gates for stable tags.'
}
if ($releaseSource -match [Regex]::Escape('${{ inputs.version }}')) {
    throw 'The release workflow still accepts a second mutable version source.'
}
if ($releaseSource -notmatch [Regex]::Escape('Test-Screenshots.ps1 -RequireComplete')) {
    throw 'The tagged release workflow does not require completed Store screenshots.'
}
if ($releaseSource -notmatch [Regex]::Escape('.audit.json')) {
    throw 'The release workflow does not stage MSIX audit evidence.'
}
$manifestSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'packaging\msix\AppxManifest.xml')
foreach ($requiredShellToken in @(
    'windows.comServer',
    'windows.fileExplorerContextMenus',
    'zifile-shell.dll',
    '2F86F25D-3B76-4CD2-8FE8-9D7A2EEFB531'
)) {
    if ($manifestSource -notmatch [Regex]::Escape($requiredShellToken)) {
        throw "The MSIX manifest does not include shell extension token: $requiredShellToken"
    }
}
if ($manifestSource -notmatch [Regex]::Escape('<uap:FileType>.rar</uap:FileType>')) {
    throw 'The MSIX manifest does not associate supported RAR archives.'
}
if ($buildSource -notmatch [Regex]::Escape('zifile_shell.dll')) {
    throw 'Build-Package.ps1 does not stage the architecture-matched shell DLL.'
}
if ($releaseSource -notmatch [Regex]::Escape('zifile-shell-windows-$arch.dll')) {
    throw 'The release workflow does not stage the standalone shell DLL.'
}
$lifecycleSource = Get-Content -Raw -LiteralPath $packageLifecycle
foreach ($requiredLifecycleToken in @(
    'ConfirmLifecycle',
    'RequireSignature',
    'Refusing to modify an existing',
    'Reset-AppxPackage',
    'Remove-AppxPackage',
    'RepairHelper',
    'preserve-application-data',
    'local_state_preserved',
    'reset_semantics'
)) {
    if ($lifecycleSource -notmatch [Regex]::Escape($requiredLifecycleToken)) {
        throw "The MSIX lifecycle gate omits required policy token: $requiredLifecycleToken"
    }
}
foreach ($requiredWorkflowToken in @(
    'baseline_run_id',
    'upgrade_run_id',
    'actions/download-artifact@v7',
    'msix-lifecycle.ps1',
    'ConfirmLifecycle',
    'msix-lifecycle-x64',
    'WindowsAppSDKSelfContained=true',
    'msix-repair-helper',
    'Invoke-Probe.ps1',
    '-RepairHelper'
)) {
    if ($lifecycleWorkflowSource -notmatch [Regex]::Escape($requiredWorkflowToken)) {
        throw "The trusted lifecycle workflow omits required token: $requiredWorkflowToken"
    }
}
$repairHelperSource = Get-Content -Raw -LiteralPath $repairHelper
$repairProjectSource = Get-Content -Raw -LiteralPath $repairProject
foreach ($requiredRepairToken in @(
    'PackageDeploymentManager.IsPackageDeploymentFeatureSupported',
    'PackageDeploymentFeature.RepairPackage',
    'RepairPackageAsync',
    'probe_completed',
    '--package-full-name',
    'preserve_application_data'
)) {
    if ($repairHelperSource -notmatch [Regex]::Escape($requiredRepairToken)) {
        throw "The MSIX Repair helper omits required token: $requiredRepairToken"
    }
}
$repairProbeSource = Get-Content -Raw -LiteralPath $repairProbe
foreach ($requiredProbeToken in @(
    'TimeoutSeconds = 15',
    'WaitForExit',
    '$process.Kill()',
    'probe_timed_out',
    'TimeoutFixture'
)) {
    if ($repairProbeSource -notmatch [Regex]::Escape($requiredProbeToken)) {
        throw "The external MSIX Repair probe omits required token: $requiredProbeToken"
    }
}
$ciSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\ci.yml')
if ($ciSource -notmatch [Regex]::Escape('./scripts/Test-VersionConsistency.ps1')) {
    throw 'CI does not enforce version consistency.'
}
if ($ciSource -notmatch [Regex]::Escape('./scripts/Test-ReleaseNotes.ps1')) {
    throw 'CI does not enforce the changelog structure.'
}
if ($ciSource -notmatch [Regex]::Escape('./scripts/Test-ContributorDocs.ps1')) {
    throw 'CI does not enforce contributor documentation consistency.'
}
if ($ciSource -notmatch [Regex]::Escape('./scripts/Test-SecurityDocs.ps1')) {
    throw 'CI does not enforce security documentation consistency.'
}
if ($ciSource -notmatch [Regex]::Escape('./scripts/Test-ReleaseReadiness.ps1')) {
    throw 'CI does not validate the 1.0 release readiness manifest.'
}
foreach ($requiredRepairWorkflowToken in @(
    'MSIX Repair helper',
    'timeout-minutes: 2',
    'Prove Repair probe hard timeout'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredRepairWorkflowToken)) {
        throw "The CI Repair probe omits required timeout token: $requiredRepairWorkflowToken"
    }
}
if ($repairProjectSource -notmatch [Regex]::Escape('Microsoft.WindowsAppSDK') -or
    $repairProjectSource -notmatch [Regex]::Escape('1.8.260804001')) {
    throw 'The MSIX Repair helper does not pin its Windows App SDK dependency.'
}
$rarCorpusSource = Get-Content -Raw -LiteralPath $rarCorpus
foreach ($requiredRarToken in @(
    '7d8f9386ef777a2415da34fe1db193d8471ff7d0',
    'api.github.com/repos/bitplane/rars/contents/crates/rars/tests/fixtures',
    'GITHUB_TOKEN',
    'git_url',
    'FromBase64String',
    'pinned-rars-golden',
    'E70E00C521EE53176D194CFC66D2C284E340D50C07667776071B220ED956570E',
    'winrar721_header_encrypted_quickopen.rar',
    'rar50/wild/symlink.rar',
    'expected_rejection',
    '$global:LASTEXITCODE = 0',
    'Assert-TreesMatch'
)) {
    if ($rarCorpusSource -notmatch [Regex]::Escape($requiredRarToken)) {
        throw "The RAR corpus gate omits required token: $requiredRarToken"
    }
}
foreach ($requiredCiToken in @(
    'RAR reference interoperability',
    'cargo build -p zifile-cli --locked',
    'rar-corpus.ps1',
    'target/rar-corpus.json'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredCiToken)) {
        throw "CI does not publish the RAR corpus gate or evidence: $requiredCiToken"
    }
}
foreach ($requiredWackToken in @(
    'WACK readiness policy',
    './tests/smoke/wack-readiness.ps1'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredWackToken)) {
        throw "CI does not exercise WACK readiness policy: $requiredWackToken"
    }
}

[pscustomobject]@{
    schema_version = 1
    parser_checks = $scriptsToParse.Count
    missing_inputs_rejected = $true
    development_identity_rejected = $true
    unsigned_publisher_rejected = $true
    formal_inputs_accepted = $true
    stable_pfx_release_rejected = $true
    malformed_release_version_rejected = $true
    package_audit_wired = $true
    release_audit_staged = $true
    shell_extension_wired = $true
    lifecycle_gate_wired = $true
    lifecycle_workflow_wired = $true
    repair_helper_wired = $true
    rar_association_wired = $true
    rar_corpus_wired = $true
    wack_readiness_wired = $true
    version_consistency_wired = $true
    release_notes_wired = $true
    contributor_docs_wired = $true
    security_docs_wired = $true
    release_readiness_wired = $true
    stable_release_pending_rejected = $true
    complete_release_readiness_accepted = $true
} | ConvertTo-Json
