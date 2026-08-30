$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$publishingPolicy = Join-Path $repoRoot 'packaging\msix\Test-PublishingInputs.ps1'
$packageAudit = Join-Path $repoRoot 'packaging\msix\Test-Package.ps1'
$packageBuild = Join-Path $repoRoot 'packaging\msix\Build-Package.ps1'
$packageBundle = Join-Path $repoRoot 'packaging\msix\Build-Bundle.ps1'
$packageLifecycle = Join-Path $repoRoot 'tests\smoke\msix-lifecycle.ps1'
$repairHelper = Join-Path $repoRoot 'tests\helpers\msix-repair\Program.cs'
$repairProject = Join-Path $repoRoot 'tests\helpers\msix-repair\MsixRepair.csproj'
$repairProbe = Join-Path $repoRoot 'tests\helpers\msix-repair\Invoke-Probe.ps1'
$rarCorpus = Join-Path $repoRoot 'tests\interoperability\rar-corpus.ps1'
$cabInteroperability = Join-Path $repoRoot 'tests\interoperability\cab-windows.ps1'
$zipMethodCorpus = Join-Path $repoRoot 'tests\interoperability\zip-method-corpus.ps1'
$zipLegacyCorpus = Join-Path $repoRoot 'tests\interoperability\zip-legacy-corpus.ps1'
$zipZstdCorpus = Join-Path $repoRoot 'tests\interoperability\zip-zstd-corpus.ps1'
$windowsTools = Join-Path $repoRoot 'tests\interoperability\windows-tools.ps1'
$contractPolicy = Join-Path $repoRoot 'tests\smoke\contract-policy.ps1'
$wackReadiness = Join-Path $repoRoot 'packaging\msix\Test-WackReadiness.ps1'
$versionConsistency = Join-Path $repoRoot 'scripts\Test-VersionConsistency.ps1'
$releaseNotes = Join-Path $repoRoot 'scripts\Test-ReleaseNotes.ps1'
$contributorDocs = Join-Path $repoRoot 'scripts\Test-ContributorDocs.ps1'
$securityDocs = Join-Path $repoRoot 'scripts\Test-SecurityDocs.ps1'
$releaseReadiness = Join-Path $repoRoot 'scripts\Test-ReleaseReadiness.ps1'
$cloudSigningInputs = Join-Path $repoRoot 'packaging\msix\Test-CloudSigningInputs.ps1'
$signedReleaseArtifacts = Join-Path $repoRoot 'packaging\msix\Test-SignedReleaseArtifacts.ps1'
$signingOperationsDocs = Join-Path $repoRoot 'scripts\Test-SigningOperationsDocs.ps1'
$partnerCenterIdentity = Join-Path $repoRoot 'packaging\store\Test-PartnerCenterIdentity.ps1'
$publicPrivacy = Join-Path $repoRoot 'packaging\store\Test-PublicPrivacy.ps1'
$wingetGenerator = Join-Path $repoRoot 'packaging\winget\Generate-Manifests.ps1'
$wingetVerifier = Join-Path $repoRoot 'packaging\winget\Test-Manifests.ps1'
$wingetClientInstaller = Join-Path $repoRoot 'packaging\winget\Install-ValidationClient.ps1'
$wingetSmoke = Join-Path $repoRoot 'tests\smoke\winget-manifest.ps1'
$userDocs = Join-Path $repoRoot 'scripts\Test-UserDocs.ps1'
$reproducibilityWorkflow = Join-Path $repoRoot '.github\workflows\reproducibility.yml'
$operationQueueForeground = Join-Path $repoRoot 'tests\performance\operation-queue-foreground.ps1'
$msixAssets = Join-Path $repoRoot 'packaging\msix\Test-Assets.ps1'
$embeddedIconAudit = Join-Path $repoRoot 'packaging\msix\Test-EmbeddedIcon.ps1'
$storeListingAssets = Join-Path $repoRoot 'packaging\store\Test-ListingAssets.ps1'
$storeListingAssetSmoke = Join-Path $repoRoot 'tests\smoke\store-listing-assets.ps1'

$scriptsToParse = @($publishingPolicy, $packageAudit, $packageBuild, $packageBundle, $packageLifecycle, $repairProbe, $rarCorpus, $cabInteroperability, $zipMethodCorpus, $zipLegacyCorpus, $zipZstdCorpus, $windowsTools, $contractPolicy, $wackReadiness, $versionConsistency, $releaseNotes, $contributorDocs, $securityDocs, $releaseReadiness, $cloudSigningInputs, $signedReleaseArtifacts, $signingOperationsDocs, $partnerCenterIdentity, $publicPrivacy, $wingetGenerator, $wingetVerifier, $wingetClientInstaller, $wingetSmoke, $userDocs, $operationQueueForeground, $msixAssets, $embeddedIconAudit, $storeListingAssets, $storeListingAssetSmoke)
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

$assetResult = & $msixAssets -VerifyGenerator | ConvertFrom-Json
if (-not $assetResult.validated -or -not $assetResult.hashes_pinned -or
    -not $assetResult.generator_matches_on_current_host -or
    $assetResult.png_count -ne 58 -or $assetResult.scale_asset_count -ne 11 -or
    $assetResult.app_list_target_asset_count -ne 42 -or
    $assetResult.app_list_target_sizes.Count -ne 14 -or
    $assetResult.app_list_theme_variants -ne 3 -or
    $assetResult.icon_frames.Count -ne 5 -or
    ($assetResult.icon_frames -join ',') -cne '16,24,32,48,256' -or
    $assetResult.icon_sha256 -notmatch '^[A-F0-9]{64}$' -or
    $assetResult.manifest_logo_references -ne 4) {
    throw 'MSIX visual assets did not pass completeness, manifest, and reproducibility validation.'
}
$resourceLessSystemBinary = Join-Path $env:SystemRoot 'System32\kernel32.dll'
if (-not (Test-Path -LiteralPath $resourceLessSystemBinary -PathType Leaf)) {
    throw 'The system resource-less binary fixture is unavailable.'
}
$null = Get-ExpectedFailure -Pattern 'does not contain reviewed GROUP_ICON resource ID 1' -Action {
    & $embeddedIconAudit -ExecutablePath $resourceLessSystemBinary
}
$storeAssetResult = & $storeListingAssetSmoke | ConvertFrom-Json
if (-not $storeAssetResult.valid_store_icon_accepted -or
    -not $storeAssetResult.missing_store_icon_rejected -or
    -not $storeAssetResult.incorrect_store_icon_dimensions_rejected -or
    -not $storeAssetResult.modified_store_icon_rejected) {
    throw 'Store listing app tile positive and negative policy did not pass.'
}
$assetTemporaryBase = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$assetFixture = [System.IO.Path]::GetFullPath((Join-Path $assetTemporaryBase (
    'zifile-asset-policy-{0}' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $assetFixture.StartsWith($assetTemporaryBase, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the asset-policy fixture outside the system temporary directory.'
}
try {
    $fixtureAssets = Join-Path $assetFixture 'Assets'
    New-Item -ItemType Directory -Path $fixtureAssets -Force | Out-Null
    Get-ChildItem -LiteralPath (Join-Path $repoRoot 'packaging\msix\Assets') -File |
        Copy-Item -Destination $fixtureAssets
    $fixtureManifest = Join-Path $assetFixture 'AppxManifest.xml'
    Copy-Item -LiteralPath (Join-Path $repoRoot 'packaging\msix\AppxManifest.xml') -Destination $fixtureManifest

    Copy-Item `
        -LiteralPath (Join-Path $fixtureAssets 'Square50x50Logo.png') `
        -Destination (Join-Path $fixtureAssets 'Square44x44Logo.png') -Force
    $null = Get-ExpectedFailure -Pattern 'must be 44x44' -Action {
        & $msixAssets -AssetsDirectory $fixtureAssets -ManifestPath $fixtureManifest
    }

    Copy-Item `
        -LiteralPath (Join-Path $repoRoot 'packaging\msix\Assets\Square44x44Logo.png') `
        -Destination (Join-Path $fixtureAssets 'Square44x44Logo.png') -Force
    $manifestSource = Get-Content -Raw -LiteralPath $fixtureManifest
    $manifestSource = $manifestSource.Replace('Assets\StoreLogo.png', 'Assets\MissingStoreLogo.png')
    Set-Content -LiteralPath $fixtureManifest -Value $manifestSource -Encoding utf8
    $null = Get-ExpectedFailure -Pattern 'logo reference does not exist' -Action {
        & $msixAssets -AssetsDirectory $fixtureAssets -ManifestPath $fixtureManifest
    }

    Copy-Item -LiteralPath (Join-Path $repoRoot 'packaging\msix\AppxManifest.xml') -Destination $fixtureManifest -Force
    $missingQualifiedAsset = Join-Path $fixtureAssets 'Square44x44Logo.targetsize-96_altform-lightunplated.png'
    Remove-Item -LiteralPath $missingQualifiedAsset -Force
    $null = Get-ExpectedFailure -Pattern 'incomplete or unexpected PNG set' -Action {
        & $msixAssets -AssetsDirectory $fixtureAssets -ManifestPath $fixtureManifest
    }
    Copy-Item `
        -LiteralPath (Join-Path $repoRoot 'packaging\msix\Assets\Square44x44Logo.targetsize-96_altform-lightunplated.png') `
        -Destination $missingQualifiedAsset

    $changedIcon = Join-Path $fixtureAssets 'ZiFile.ico'
    $changedIconBytes = [IO.File]::ReadAllBytes($changedIcon)
    $changedIconBytes[4] = 1
    $changedIconBytes[5] = 0
    [IO.File]::WriteAllBytes($changedIcon, $changedIconBytes)
    $null = Get-ExpectedFailure -Pattern 'must contain exactly 5 frames' -Action {
        & $msixAssets -AssetsDirectory $fixtureAssets -ManifestPath $fixtureManifest
    }
    Copy-Item `
        -LiteralPath (Join-Path $repoRoot 'packaging\msix\Assets\ZiFile.ico') `
        -Destination $changedIcon -Force

    Add-Type -AssemblyName System.Drawing
    $changedAsset = Join-Path $fixtureAssets 'Square50x50Logo.png'
    $loadedBitmap = [Drawing.Bitmap]::new($changedAsset)
    try { $changedBitmap = [Drawing.Bitmap]::new($loadedBitmap) }
    finally { $loadedBitmap.Dispose() }
    try {
        $changedBitmap.SetPixel(0, 0, [Drawing.Color]::FromArgb(255, 255, 0, 0))
        $changedBitmap.Save($changedAsset, [Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $changedBitmap.Dispose() }
    $null = Get-ExpectedFailure -Pattern 'hash does not match its reviewed pinned value' -Action {
        & $msixAssets -AssetsDirectory $fixtureAssets -ManifestPath $fixtureManifest
    }
}
finally {
    if (Test-Path -LiteralPath $assetFixture) {
        $resolvedAssetFixture = [System.IO.Path]::GetFullPath($assetFixture)
        if (-not $resolvedAssetFixture.StartsWith($assetTemporaryBase, [System.StringComparison]::OrdinalIgnoreCase) -or
            [System.IO.Path]::GetFileName($resolvedAssetFixture) -notlike 'zifile-asset-policy-*') {
            throw "Refusing to remove unexpected asset-policy fixture: $resolvedAssetFixture"
        }
        Remove-Item -LiteralPath $resolvedAssetFixture -Recurse -Force
    }
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

$wingetFixture = [System.IO.Path]::GetFullPath((Join-Path $temporaryBase (
    'zifile-winget-policy-{0}' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $wingetFixture.StartsWith($temporaryBase, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the WinGet fixture outside the system temporary directory.'
}
try {
    New-Item -ItemType Directory -Path $wingetFixture -Force | Out-Null
    $wingetVersion = '9.8.7'
    $wingetX64 = Join-Path $wingetFixture 'ZiFile-9.8.7.0-windows-x64.msix'
    $wingetArm64 = Join-Path $wingetFixture 'ZiFile-9.8.7.0-windows-arm64.msix'
    Set-Content -LiteralPath $wingetX64 -Value 'deterministic x64 signed-package fixture' -Encoding utf8NoBOM
    Set-Content -LiteralPath $wingetArm64 -Value 'deterministic arm64 signed-package fixture' -Encoding utf8NoBOM
    $wingetX64Sha = (Get-FileHash -LiteralPath $wingetX64 -Algorithm SHA256).Hash
    $wingetArm64Sha = (Get-FileHash -LiteralPath $wingetArm64 -Algorithm SHA256).Hash
    & $wingetGenerator `
        -Version $wingetVersion `
        -X64InstallerUrl "https://github.com/ax2/zifile/releases/download/v$wingetVersion/$(Split-Path $wingetX64 -Leaf)" `
        -X64InstallerSha256 $wingetX64Sha `
        -Arm64InstallerUrl "https://github.com/ax2/zifile/releases/download/v$wingetVersion/$(Split-Path $wingetArm64 -Leaf)" `
        -Arm64InstallerSha256 $wingetArm64Sha `
        -OutputRoot $wingetFixture | Out-Null
    $wingetManifestDirectory = Join-Path $wingetFixture 'manifests\z\ZiCode\ZiFile\9.8.7'
    foreach ($manifestPath in Get-ChildItem -LiteralPath $wingetManifestDirectory -File -Filter '*.yaml') {
        $crlfSource = (Get-Content -Raw -LiteralPath $manifestPath.FullName).Replace("`r`n", "`n").Replace("`n", "`r`n")
        Set-Content -LiteralPath $manifestPath.FullName -Value $crlfSource -Encoding utf8NoBOM -NoNewline
    }
    $wingetResult = & $wingetVerifier `
        -ManifestDirectory $wingetManifestDirectory `
        -Version $wingetVersion `
        -X64InstallerPath $wingetX64 `
        -Arm64InstallerPath $wingetArm64 | ConvertFrom-Json
    if (-not $wingetResult.ready_for_winget_validate -or
        -not $wingetResult.local_installers_verified -or
        $wingetResult.manifest_files -ne 4 -or
        $wingetResult.architectures.Count -ne 2) {
        throw 'The generated WinGet multi-file candidate did not pass the signed-package verifier.'
    }
    $installerManifest = Join-Path $wingetManifestDirectory 'ZiCode.ZiFile.installer.yaml'
    $originalInstallerManifest = Get-Content -Raw -LiteralPath $installerManifest
    $tamperedInstallerManifest = $originalInstallerManifest.Replace($wingetX64Sha, ('0' * 64))
    Set-Content -LiteralPath $installerManifest -Value $tamperedInstallerManifest -Encoding utf8NoBOM -NoNewline
    $null = Get-ExpectedFailure -Pattern 'does not match the signed local MSIX' -Action {
        & $wingetVerifier `
            -ManifestDirectory $wingetManifestDirectory `
            -Version $wingetVersion `
            -X64InstallerPath $wingetX64 `
            -Arm64InstallerPath $wingetArm64
    }
    $null = Get-ExpectedFailure -Pattern 'versioned ZiFile GitHub Release path' -Action {
        & $wingetGenerator `
            -Version $wingetVersion `
            -X64InstallerUrl "https://example.com/$(Split-Path $wingetX64 -Leaf)" `
            -X64InstallerSha256 $wingetX64Sha `
            -Arm64InstallerUrl "https://github.com/ax2/zifile/releases/download/v$wingetVersion/$(Split-Path $wingetArm64 -Leaf)" `
            -Arm64InstallerSha256 $wingetArm64Sha `
            -OutputRoot $wingetFixture
    }
}
finally {
    if (Test-Path -LiteralPath $wingetFixture) {
        Remove-Item -LiteralPath $wingetFixture -Recurse -Force
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
$userDocsResult = & $userDocs | ConvertFrom-Json
if (-not $userDocsResult.synchronized -or $userDocsResult.locale_pairs -ne 2 -or
    -not $userDocsResult.navigation_wired -or -not $userDocsResult.safe_reporting_documented) {
    throw 'User documentation is not synchronized with product and safety boundaries.'
}
$signingOperationsResult = & $signingOperationsDocs | ConvertFrom-Json
if (-not $signingOperationsResult.synchronized -or
    $signingOperationsResult.locale_pages -ne 2 -or
    -not $signingOperationsResult.least_privilege_workflow -or
    -not $signingOperationsResult.emergency_stop_runbook) {
    throw 'Signing operations documentation is not synchronized with the protected workflow.'
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
"@ | Set-Content -LiteralPath (Join-Path $releaseFixture 'CHANGELOG.md') -Encoding utf8NoBOM
    $null = Get-ExpectedFailure -Pattern 'does not contain release heading' -Action {
        & $releaseNotes -RepositoryRoot $releaseFixture -ExpectedVersion $versionResult.tag
    }
    @"

## [$($versionResult.version)] - 2026-08-26

### Added

- Candidate release evidence.
"@ | Add-Content -LiteralPath (Join-Path $releaseFixture 'CHANGELOG.md') -Encoding utf8NoBOM
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
    & $publishingPolicy -IdentityName '' -Publisher '' -PublisherDisplayName ''
}
foreach ($secretName in @(
    'ZIFILE_MSIX_IDENTITY',
    'ZIFILE_MSIX_PUBLISHER',
    'ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME',
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
        -PublisherDisplayName 'ZiCode' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}

$unconfiguredIdentity = & $partnerCenterIdentity | ConvertFrom-Json
if ($unconfiguredIdentity.configured -or $unconfiguredIdentity.formal_identity) {
    throw 'An absent Partner Center identity was not reported as unconfigured.'
}
$null = Get-ExpectedFailure -Pattern 'ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME' -Action {
    & $partnerCenterIdentity -RequireConfigured
}
$null = Get-ExpectedFailure -Pattern 'configured together' -Action {
    & $partnerCenterIdentity -IdentityName 'ZiCode.ZiFile'
}
$null = Get-ExpectedFailure -Pattern '3-50 alphanumeric' -Action {
    & $partnerCenterIdentity -IdentityName 'ZiCode ZiFile' -Publisher 'CN=ZiCode Official' -PublisherDisplayName 'ZiCode'
}
$null = Get-ExpectedFailure -Pattern 'development MSIX identity' -Action {
    & $partnerCenterIdentity -IdentityName 'ZiCode.ZiFile.Dev' -Publisher 'CN=ZiCode Official' -PublisherDisplayName 'ZiCode'
}
$null = Get-ExpectedFailure -Pattern 'unsigned development publisher' -Action {
    & $partnerCenterIdentity `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Development, OID.2.25.311729368913984317654407730594956997722=1' `
        -PublisherDisplayName 'ZiCode'
}
$null = Get-ExpectedFailure -Pattern 'X.500 distinguished name' -Action {
    & $partnerCenterIdentity -IdentityName 'ZiCode.ZiFile' -Publisher 'not a distinguished name' -PublisherDisplayName 'ZiCode'
}
$null = Get-ExpectedFailure -Pattern '1-256 printable characters' -Action {
    & $partnerCenterIdentity -IdentityName 'ZiCode.ZiFile' -Publisher 'CN=ZiCode Official' -PublisherDisplayName "ZiCode`nOfficial"
}
$formalIdentity = & $partnerCenterIdentity `
    -IdentityName 'ZiCode.ZiFile' `
    -Publisher 'CN=ZiCode Official, O=ZiCode' `
    -PublisherDisplayName 'ZiCode' `
    -RequireConfigured |
    ConvertFrom-Json
if (-not $formalIdentity.configured -or -not $formalIdentity.formal_identity -or
    $formalIdentity.publisher_display_name -cne 'ZiCode') {
    throw 'A valid Partner Center product identity was not accepted.'
}
$null = Get-ExpectedFailure -Pattern 'unsigned development publisher' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Development, OID.2.25.311729368913984317654407730594956997722=1' `
        -PublisherDisplayName 'ZiCode' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}
$accepted = & $publishingPolicy `
    -IdentityName 'ZiCode.ZiFile' `
    -Publisher 'CN=ZiCode Official' `
    -PublisherDisplayName 'ZiCode' `
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
        -PublisherDisplayName 'ZiCode' `
        -ReleaseVersion 'v1.0.0' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}
$null = Get-ExpectedFailure -Pattern 'supported semantic version' -Action {
    & $publishingPolicy `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Official' `
        -PublisherDisplayName 'ZiCode' `
        -ReleaseVersion 'release' `
        -SigningCertificateAvailable `
        -SigningPasswordAvailable
}

$cloudMissingFailure = Get-ExpectedFailure -Pattern 'protected production inputs' -Action {
    & $cloudSigningInputs -Provider '' -IdentityName '' -Publisher ''
}
foreach ($inputName in @(
    'ZIFILE_SIGNING_PROVIDER',
    'ZIFILE_MSIX_IDENTITY',
    'ZIFILE_MSIX_PUBLISHER',
    'SM_HOST',
    'SM_API_KEY',
    'SM_CLIENT_CERT_FILE_B64',
    'SM_CLIENT_CERT_PASSWORD',
    'SM_KEYPAIR_ALIAS'
)) {
    if ($cloudMissingFailure -notmatch [Regex]::Escape($inputName)) {
        throw "Cloud-signing input diagnostic omitted $inputName."
    }
}
$cloudAccepted = & $cloudSigningInputs `
    -Provider 'digicert-stm' `
    -IdentityName 'ZiCode.ZiFile' `
    -Publisher 'CN=ZiCode Official' `
    -HostAvailable `
    -ApiKeyAvailable `
    -ClientCertificateAvailable `
    -ClientCertificatePasswordAvailable `
    -KeypairAliasAvailable |
    ConvertFrom-Json
if (-not $cloudAccepted.validated -or
    $cloudAccepted.private_code_signing_key_exported -or
    $cloudAccepted.credential_values_disclosed) {
    throw 'Protected cloud-signing inputs were not accepted with the required custody boundary.'
}
$null = Get-ExpectedFailure -Pattern 'Unsupported production signing provider' -Action {
    & $cloudSigningInputs `
        -Provider 'pfx' `
        -IdentityName 'ZiCode.ZiFile' `
        -Publisher 'CN=ZiCode Official' `
        -HostAvailable `
        -ApiKeyAvailable `
        -ClientCertificateAvailable `
        -ClientCertificatePasswordAvailable `
        -KeypairAliasAvailable
}
$null = Get-ExpectedFailure -Pattern 'development MSIX identity' -Action {
    & $cloudSigningInputs `
        -Provider 'digicert-stm' `
        -IdentityName 'ZiCode.ZiFile.Dev' `
        -Publisher 'CN=ZiCode Official' `
        -HostAvailable `
        -ApiKeyAvailable `
        -ClientCertificateAvailable `
        -ClientCertificatePasswordAvailable `
        -KeypairAliasAvailable
}

$signedFixture = [System.IO.Path]::GetFullPath((Join-Path $temporaryBase (
    'zifile-signed-artifacts-{0}' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $signedFixture.StartsWith($temporaryBase, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to create the signed-artifact fixture outside the system temporary directory.'
}
try {
    New-Item -ItemType Directory -Path $signedFixture -Force | Out-Null
    foreach ($fixtureName in @(
        'zifile-windows-x64.exe',
        'zifile-cli-windows-x64.exe',
        'zifile-worker-windows-x64.exe',
        'zifile-shell-windows-x64.dll',
        'ZiFile-1.0.0.0-windows-x64.msix'
    )) {
        Set-Content -LiteralPath (Join-Path $signedFixture $fixtureName) `
            -Value 'intentionally unsigned fixture' -Encoding utf8NoBOM
    }
    $null = Get-ExpectedFailure -Pattern 'signature is not valid' -Action {
        & $signedReleaseArtifacts `
            -ArtifactDirectory $signedFixture `
            -Architecture x64 `
            -ExpectedVersion 1.0.0.0 `
            -ExpectedIdentityName 'ZiCode.ZiFile' `
            -ExpectedPublisher 'CN=ZiCode Official' `
            -ExpectedPublisherDisplayName 'ZiCode' `
            -Provider digicert-stm
    }
}
finally {
    if (Test-Path -LiteralPath $signedFixture) {
        Remove-Item -LiteralPath $signedFixture -Recurse -Force
    }
}

$buildSource = Get-Content -Raw -LiteralPath $packageBuild
if ($buildSource -notmatch [Regex]::Escape("Test-Package.ps1")) {
    throw 'Build-Package.ps1 does not invoke the package auditor.'
}
$releaseSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\release.yml')
$lifecycleWorkflowSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\msix-lifecycle.yml')
if ($releaseSource -notmatch [Regex]::Escape('Free code signing provided by SignPath.io, certificate by SignPath Foundation.')) {
    throw 'The stage release workflow must preserve the SignPath Foundation attribution in generated prerelease notes.'
}
foreach ($requiredSigningWorkflowToken in @(
    'signing_provider:',
    'environment: production-signing',
    'Test-CloudSigningInputs.ps1',
    'Test-PartnerCenterIdentity.ps1',
    'ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME',
    'digicert/code-signing-software-trust-action@v1.2.1',
    'simple-signing-mode: true',
    'digest-alg: SHA-256',
    'timestamp: true',
    'Test-SignedReleaseArtifacts.ps1',
    'signed-windows-${{ matrix.architecture }}',
    'Attest signed Windows artifacts',
    'Refresh signed artifact checksums',
    'Where-Object { $_.FullName -ne $checksum }'
)) {
    if ($releaseSource -notmatch [Regex]::Escape($requiredSigningWorkflowToken)) {
        throw "The release workflow omits cloud-signing token: $requiredSigningWorkflowToken"
    }
}
foreach ($requiredBundleWorkflowToken in @(
    'Build-Bundle.ps1',
    'Windows all-in-one MSIX bundle',
    'windows-all-in-one',
    '.msixbundle',
    'public-release/*'
)) {
    if ($releaseSource -notmatch [Regex]::Escape($requiredBundleWorkflowToken)) {
        throw "The release workflow omits all-in-one installer token: $requiredBundleWorkflowToken"
    }
}
foreach ($requiredPortableSmokeToken in @(
    'Smoke test standalone desktop executable',
    'tests/smoke/portable-exe.ps1'
)) {
    if ($releaseSource -notmatch [Regex]::Escape($requiredPortableSmokeToken)) {
        throw "The release workflow omits standalone portable EXE smoke coverage: $requiredPortableSmokeToken"
    }
}
$portableSmokeSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'tests\smoke\portable-exe.ps1')
foreach ($requiredPortableSmokeSourceToken in @(
    '--zifile-worker',
    'separate_worker_present',
    'archive_entry'
)) {
    if ($portableSmokeSource -notmatch [Regex]::Escape($requiredPortableSmokeSourceToken)) {
        throw "The standalone portable EXE smoke test omits required behavior: $requiredPortableSmokeSourceToken"
    }
}
foreach ($retiredPfxToken in @('ZIFILE_PFX_BASE64', 'ZIFILE_PFX_PASSWORD')) {
    if ($releaseSource -match [Regex]::Escape($retiredPfxToken)) {
        throw "The release workflow still references retired PFX input: $retiredPfxToken"
    }
}
$signedVerifierSource = Get-Content -Raw -LiteralPath $signedReleaseArtifacts
foreach ($requiredVerifierToken in @(
    'Get-AuthenticodeSignature',
    'SignatureStatus]::Valid',
    'TimeStamperCertificate',
    'SignerCertificate.Subject -ne $ExpectedPublisher',
    'ExpectedPublisherDisplayName',
    'RequireSignature',
    'SHA256SUMS-$Architecture.txt',
    '*.zip'
)) {
    if ($signedVerifierSource -notmatch [Regex]::Escape($requiredVerifierToken)) {
        throw "The signed-release verifier omits required token: $requiredVerifierToken"
    }
}
if ($releaseSource -notmatch [Regex]::Escape('Test-VersionConsistency.ps1 @arguments')) {
    throw 'The release workflow does not enforce the workspace version source.'
}
if ($releaseSource -notmatch [Regex]::Escape('Test-ReleaseNotes.ps1 @arguments')) {
    throw 'The release workflow does not enforce versioned release notes.'
}
if ($releaseSource -notmatch [Regex]::Escape('Test-ReleaseReadiness.ps1 @readinessArguments') -or
    $releaseSource -notmatch [Regex]::Escape('$readinessArguments.RequireReleaseReady = $true')) {
    throw 'The release workflow does not retain the explicit formal-readiness option.'
}
if ($releaseSource -match [Regex]::Escape('${{ inputs.version }}')) {
    throw 'The release workflow still accepts a second mutable version source.'
}
if ($releaseSource -notmatch [Regex]::Escape('Test-Screenshots.ps1 -RequireComplete')) {
    throw 'The tagged release workflow does not require completed Store screenshots.'
}
foreach ($requiredStageReleaseToken in @(
    "if: startsWith(github.ref, 'refs/tags/v') && contains(github.ref_name, '-')",
    'publish-stage:',
    'needs: [windows, bundle, sbom]',
    'pattern: windows-*',
    'prerelease: true',
    'Prepare user-facing stage release assets',
    '*.msixbundle',
    'zifile-windows-x64.exe',
    'zifile-windows-arm64.exe',
    'sha256sum ./*.msixbundle ./*.exe',
    'public-release/*'
)) {
    if ($releaseSource -notmatch [Regex]::Escape($requiredStageReleaseToken)) {
        throw "The release workflow omits stage-release token: $requiredStageReleaseToken"
    }
}
if ($releaseSource -match [Regex]::Escape('sha256sum release/* > release/SHA256SUMS-stage.txt')) {
    throw 'The stage release checksum manifest must not hash itself.'
}
if ($releaseSource -notmatch [Regex]::Escape("if: inputs.signing_provider == 'digicert-stm'")) {
    throw 'The production signing job must be limited to an explicit manual signing rehearsal.'
}
if ($releaseSource -match [Regex]::Escape("if: startsWith(github.ref, 'refs/tags/v') || inputs.signing_provider == 'digicert-stm'")) {
    throw 'The production signing job must not run automatically for prerelease tags.'
}
if ($releaseSource -notmatch [Regex]::Escape('.audit.json')) {
    throw 'The release workflow does not stage MSIX audit evidence.'
}
foreach ($requiredIdentitySelectionToken in @(
    '$useFormalIdentity',
    ([char]39 + '${{ inputs.signing_provider }}' + [char]39 + ' -eq ' + [char]39 + 'digicert-stm' + [char]39),
    ([char]39 + '${{ inputs.require_release_ready }}' + [char]39 + ' -eq ' + [char]39 + 'true' + [char]39),
    'if ($useFormalIdentity)',
    'Build-Package.ps1 @arguments'
)) {
    if ($releaseSource -notmatch [Regex]::Escape($requiredIdentitySelectionToken)) {
        throw "The release workflow does not guard formal MSIX identity selection: $requiredIdentitySelectionToken"
    }
}
foreach ($requiredPublicReleaseToken in @(
    'needs: [windows, bundle, sbom]',
    'pattern: windows-*',
    'prerelease: false',
    'No all-in-one MSIX bundle was staged for the public release.',
    'Expected one standalone x64 and one standalone ARM64 portable executable for the public release.',
    'zifile-windows-x64.exe',
    'zifile-windows-arm64.exe',
    'public-release',
    'files: public-release/*',
    'public unsigned Windows build',
    'Verify the included SHA256SUMS file'
)) {
    if ($releaseSource -notmatch [Regex]::Escape($requiredPublicReleaseToken)) {
        throw "The stable GitHub release does not publish the unsigned public build: $requiredPublicReleaseToken"
    }
}
$manifestSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'packaging\msix\AppxManifest.xml')
foreach ($requiredShellToken in @(
    'windows.comServer',
    'windows.fileExplorerContextMenus',
    'Directory\Background',
    'zifile-shell.dll',
    '2F86F25D-3B76-4CD2-8FE8-9D7A2EEFB531',
    '2D39AD2E-1B36-4F4F-8E09-589F0B1D2BC3',
    'ExtractArchiveWithZiFile'
)) {
    if ($manifestSource -notmatch [Regex]::Escape($requiredShellToken)) {
        throw "The MSIX manifest does not include shell extension token: $requiredShellToken"
    }
}
if ($manifestSource -notmatch [Regex]::Escape('<uap:FileType>.rar</uap:FileType>')) {
    throw 'The MSIX manifest does not associate supported RAR archives.'
}

function Assert-WorkflowJobTimeout {
    param(
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string]$Job,
        [Parameter(Mandatory)][int]$Minutes,
        [Parameter(Mandatory)][string]$Workflow
    )
    $pattern = '(?ms)^  {0}:\r?\n(?:(?!^  [A-Za-z0-9_-]+:\r?$).)*?^    timeout-minutes: {1}\r?$' -f
        [Regex]::Escape($Job), $Minutes
    if ($Source -notmatch $pattern) {
        throw "$Workflow job '$Job' does not have the required $Minutes-minute hard timeout."
    }
}
if ($manifestSource -notmatch [Regex]::Escape('<uap:FileType>.cab</uap:FileType>')) {
    throw 'The MSIX manifest does not associate supported CAB archives.'
}
if ($manifestSource -notmatch [Regex]::Escape('<uap:FileType>.zipx</uap:FileType>')) {
    throw 'The MSIX manifest does not associate supported ZIPX archives.'
}
$packageAuditSource = Get-Content -Raw -LiteralPath $packageAudit
foreach ($requiredIdentityAuditToken in @('ExpectedPublisherDisplayName', 'publisher_display_name')) {
    if ($packageAuditSource -notmatch [Regex]::Escape($requiredIdentityAuditToken)) {
        throw "The package audit omits Store publisher display-name evidence: $requiredIdentityAuditToken"
    }
}
if ($packageAuditSource -notmatch [Regex]::Escape("'.zipx'") -or
    $packageAuditSource -notmatch [Regex]::Escape("'.tbz2'")) {
    throw 'The package audit does not require the archive alias associations.'
}
foreach ($requiredExtractAuditToken in @('$extractShellClsid', 'extract_item_types')) {
    if ($packageAuditSource -notmatch [Regex]::Escape($requiredExtractAuditToken)) {
        throw "The package audit omits Explorer extract evidence: $requiredExtractAuditToken"
    }
}
foreach ($requiredVisualAssetAuditToken in @(
    "Join-Path `$PSScriptRoot 'assets.json'",
    'MSIX package is missing reviewed visual asset',
    'MSIX package visual asset differs from its reviewed hash',
    'reviewed_visual_assets',
    'MSIX package is missing reviewed desktop icon',
    'MSIX package desktop icon differs from its reviewed hash',
    'reviewed_desktop_icon',
    "Join-Path `$PSScriptRoot 'Test-EmbeddedIcon.ps1'",
    'Packaged desktop executable did not pass the reviewed embedded icon audit',
    'embedded_desktop_icon'
)) {
    if ($packageAuditSource -notmatch [Regex]::Escape($requiredVisualAssetAuditToken)) {
        throw "The package audit omits reviewed high-DPI asset evidence: $requiredVisualAssetAuditToken"
    }
}
$shellSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'apps\zifile-shell\src\lib.rs')
foreach ($requiredExtractShellToken in @(
    'EXTRACT_COMMAND_CLSID',
    '--extract-here',
    'ECS_HIDDEN',
    'E_PENDING',
    'ok_to_be_slow',
    'shell_icon_resource',
    'zifile-desktop.exe,0',
    'detect_format',
    'path.is_file()',
    'symlink_metadata',
    'file_type().is_symlink()',
    'WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT',
    'file_attributes()',
    'is_real_file_or_directory'
)) {
    if ($shellSource -notmatch [Regex]::Escape($requiredExtractShellToken)) {
        throw "The Rust shell extension omits extract command behavior: $requiredExtractShellToken"
    }
}
foreach ($requiredShellLifetimeToken in @(
    'LIVE_OBJECTS',
    'SERVER_LOCKS',
    'update_server_locks',
    'unload_result',
    'DllCanUnloadNow'
)) {
    if ($shellSource -notmatch [Regex]::Escape($requiredShellLifetimeToken)) {
        throw "The Rust shell extension omits COM lifetime accounting: $requiredShellLifetimeToken"
    }
}
foreach ($requiredBackgroundShellToken in @(
    'IObjectWithSite',
    'IServiceProvider',
    'IShellBrowser',
    'IFolderView',
    'SID_STopLevelBrowser',
    'current_folder_path',
    'create_sources',
    'deduplicate_paths',
    'paths_have_same_identity',
    'validate_create_paths'
)) {
    if ($shellSource -notmatch [Regex]::Escape($requiredBackgroundShellToken)) {
        throw "The Rust shell extension omits folder-background resolution: $requiredBackgroundShellToken"
    }
}
if ($shellSource -match [Regex]::Escape('EXTRACT_ARCHIVE_EXTENSIONS')) {
    throw 'The Explorer extract command must reuse the core extension registry instead of a duplicate allowlist.'
}
$startupSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'apps\zifile-desktop\src\startup.rs')
if ($startupSource -notmatch [Regex]::Escape('ExtractHere') -or
    $startupSource -notmatch [Regex]::Escape('--extract-here')) {
    throw 'The shared desktop startup parser omits Explorer extract mode.'
}
$coreSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'crates\zifile-core\src\lib.rs')
$coreArchiveSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'crates\zifile-core\src\archive.rs')
$createValidationSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'apps\zifile-desktop\src\create_validation.rs')
if ($coreSource -notmatch [Regex]::Escape('OPEN_ARCHIVE_EXTENSIONS')) {
    throw 'The core does not expose the shared desktop extension registry.'
}
foreach ($requiredCreateValidationToken in @(
    'CreateSourceIssue::LinkSource',
    'is_link_like',
    'symlink_metadata',
    'file_type().is_symlink()',
    'WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT',
    'file_attributes()'
)) {
    if ($createValidationSource -notmatch [Regex]::Escape($requiredCreateValidationToken)) {
        throw "The desktop create preflight omits link-like source protection: $requiredCreateValidationToken"
    }
}
foreach ($requiredDestinationSafetyToken in @(
    'reject_symlink_components',
    'metadata_is_link_like',
    'file_attributes',
    'WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT'
)) {
    if ($coreArchiveSource -notmatch [Regex]::Escape($requiredDestinationSafetyToken)) {
        throw "Core extraction destination safety omits: $requiredDestinationSafetyToken"
    }
}
foreach ($requiredCoreExtension in @(
    'zip', 'zipx', 'cbz', 'epub', '7z', 'cb7', 'rar', 'cbr', 'cab', 'tar', 'cbt',
    'gz', 'tar.gz', 'tgz', 'zst', 'tar.zst', 'tzst', 'xz', 'tar.xz', 'txz', 'tar.lzma', 'lzma',
    'bz', 'bz2', 'tar.bz2', 'tbz', 'tbz2',
    'lz4', 'br'
)) {
    if ($coreSource -notmatch [Regex]::Escape(('"{0}"' -f $requiredCoreExtension))) {
        throw "The shared desktop extension registry omits: $requiredCoreExtension"
    }
}
if ($coreSource -notmatch [Regex]::Escape('COMPOUND_ARCHIVE_EXTENSIONS') -or
    $startupSource -notmatch [Regex]::Escape('COMPOUND_ARCHIVE_EXTENSIONS')) {
    throw 'Core format detection and desktop extraction naming must share the compound archive registry.'
}
foreach ($desktopSourcePath in @(
    (Join-Path $repoRoot 'apps\zifile-desktop\src\main.rs'),
    (Join-Path $repoRoot 'apps\zifile-desktop\src\accessible_main.rs')
)) {
    $desktopSource = Get-Content -Raw -LiteralPath $desktopSourcePath
    if ($desktopSource -notmatch [Regex]::Escape('OPEN_ARCHIVE_EXTENSIONS')) {
        throw "The desktop archive dialog does not use the shared extension registry: $desktopSourcePath"
    }
    if ($desktopSource -notmatch [Regex]::Escape('is_openable_archive_path')) {
        throw "The desktop drop handler does not use shared signature-first classification: $desktopSourcePath"
    }
    if ($desktopSource -notmatch [Regex]::Escape('append_unique_paths')) {
        throw "The desktop source handler does not use shared Windows-aware path deduplication: $desktopSourcePath"
    }
    if ($desktopSourcePath -match '\\main\.rs$' -and
        ($desktopSource -notmatch [Regex]::Escape('Task::perform') -or
         $desktopSource -notmatch [Regex]::Escape('FileDropClassified'))) {
        throw "The default desktop drop probe must run through an asynchronous Task: $desktopSourcePath"
    }
    if ($desktopSourcePath -match '\\accessible_main\.rs$' -and
        ($desktopSource -notmatch [Regex]::Escape('spawn_blocking') -or
         $desktopSource -notmatch [Regex]::Escape('handle_classified_drop'))) {
        throw "The accessible desktop drop probe must run through spawn_blocking: $desktopSourcePath"
    }
    if ($desktopSource -notmatch [Regex]::Escape('automatic_extract_destination') -or
        $desktopSource -notmatch [Regex]::Escape('extraction_destination')) {
        throw "The desktop does not implement the Explorer extract startup workflow: $desktopSourcePath"
    }
}
$desktopLibrarySource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'apps\zifile-desktop\src\lib.rs')
foreach ($requiredDropClassificationToken in @(
    'detect_format(path)',
    'detect_format_from_path(path)',
    'format.capabilities().list',
    'path.is_file()'
)) {
    if ($desktopLibrarySource -notmatch [Regex]::Escape($requiredDropClassificationToken)) {
        throw "The shared desktop drop classifier omits signature-first behavior: $requiredDropClassificationToken"
    }
}
foreach ($requiredSourceIdentityToken in @(
    'append_unique_paths',
    'paths_have_same_identity',
    'to_lowercase'
)) {
    if ($desktopLibrarySource -notmatch [Regex]::Escape($requiredSourceIdentityToken)) {
        throw "The shared desktop source deduplication omits Windows path identity behavior: $requiredSourceIdentityToken"
    }
}
foreach ($requiredAssociation in @('.cbz', '.cb7', '.cbr', '.cbt', '.tzst', '.txz', '.lzma', '.bz', '.tbz', '.tbz2')) {
    if ($manifestSource -notmatch [Regex]::Escape("<uap:FileType>$requiredAssociation</uap:FileType>")) {
        throw "The MSIX manifest omits supported archive alias: $requiredAssociation"
    }
    if ($packageAuditSource -notmatch [Regex]::Escape("'$requiredAssociation'")) {
        throw "The package audit omits supported archive alias: $requiredAssociation"
    }
}
if ($manifestSource -match [Regex]::Escape('<uap:FileType>.tar.lzma</uap:FileType>')) {
    throw 'The MSIX manifest must not declare the compound .tar.lzma suffix; Appx FileType accepts one extension component.'
}
if ($manifestSource -match [Regex]::Escape('<uap:FileType>.epub</uap:FileType>')) {
    throw 'The MSIX manifest must not take over EPUB by default.'
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
$signedLifecycleArtifactMatches = @(
    [Regex]::Matches($lifecycleWorkflowSource, '(?m)^\s+name: signed-windows-x64\s*$')
)
if ($signedLifecycleArtifactMatches.Count -ne 2 -or
    $lifecycleWorkflowSource -match '(?m)^\s+name: windows-x64\s*$') {
    throw 'Trusted lifecycle must download exactly two signed-windows-x64 artifacts and no unsigned windows-x64 artifact.'
}
$operationQueueForegroundSource = Get-Content -Raw -LiteralPath $operationQueueForeground
foreach ($requiredForegroundQueueToken in @(
    'SetForegroundWindow',
    'GetForegroundWindow',
    'ForegroundTimeoutSeconds = 3',
    'Refusing to run the foreground queue smoke because ZiFile is not the foreground window.',
    'foreground_window_verified = $true',
    '<no UI Automation document text observed>',
    '[Math]::Min(500, $normalized.Length)',
    'WorkerDelayMilliseconds',
    'ZIFILE_TEST_WORKER_DELAY_MS',
    'Get-ButtonDiagnostics',
    '[Math]::Min($buttons.Count, 32)'
)) {
    if ($operationQueueForegroundSource -notmatch [Regex]::Escape($requiredForegroundQueueToken)) {
        throw "The foreground operation-queue smoke omits required ownership token: $requiredForegroundQueueToken"
    }
}
$ciSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\ci.yml')
foreach ($ciTimeout in @(
    @{ Job = 'rust'; Minutes = 45 },
    @{ Job = 'rar-interoperability'; Minutes = 30 },
    @{ Job = 'performance'; Minutes = 30 },
    @{ Job = 'licenses'; Minutes = 15 },
    @{ Job = 'fuzz'; Minutes = 20 },
    @{ Job = 'repair-helper'; Minutes = 15 },
    @{ Job = 'docs'; Minutes = 15 }
)) {
    Assert-WorkflowJobTimeout -Source $ciSource -Job $ciTimeout.Job -Minutes $ciTimeout.Minutes -Workflow 'CI'
}
foreach ($requiredPerformanceToken in @(
    'Rust performance benchmarks',
    'cargo bench -p zifile-core --bench format_detection --locked',
    'cargo bench -p zifile-core --bench archive_throughput --locked',
    '--sample-size 10 --save-baseline ci',
    'name: rust-performance',
    'target/benchmarks/*',
    'target/criterion',
    'if: always()'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredPerformanceToken)) {
        throw "CI does not execute or retain performance evidence: $requiredPerformanceToken"
    }
}
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
if ($ciSource -notmatch [Regex]::Escape('./scripts/Test-UserDocs.ps1')) {
    throw 'CI does not enforce user documentation consistency.'
}
if ($ciSource -notmatch [Regex]::Escape('./scripts/Test-ReleaseReadiness.ps1')) {
    throw 'CI does not validate the 1.0 release readiness manifest.'
}
foreach ($requiredWingetCiToken in @(
    'Install current WinGet validation client',
    'GITHUB_TOKEN: ${{ github.token }}',
    './packaging/winget/Install-ValidationClient.ps1',
    'Official WinGet manifest validation',
    './tests/smoke/winget-manifest.ps1'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredWingetCiToken)) {
        throw "CI does not run official WinGet manifest validation: $requiredWingetCiToken"
    }
}
$wingetClientInstallerSource = Get-Content -Raw -LiteralPath $wingetClientInstaller
foreach ($requiredWingetClientToken in @(
    "[string]`$ModuleVersion = '1.29.280'",
    "[string]`$ClientVersion = '1.29.280'",
    'Install-Module',
    'Microsoft.WinGet.Client',
    'Repair-WinGetPackageManager -Version $ClientVersion -Force',
    'current_stable_client_pinned = $true'
)) {
    if ($wingetClientInstallerSource -notmatch [Regex]::Escape($requiredWingetClientToken)) {
        throw "WinGet validation client bootstrap is not pinned to the reviewed Microsoft release: $requiredWingetClientToken"
    }
}
$toolchainIndex = $ciSource.IndexOf('uses: dtolnay/rust-toolchain@1.93.0', [StringComparison]::Ordinal)
$packagingPolicyIndex = $ciSource.IndexOf('name: Packaging policy smoke test', [StringComparison]::Ordinal)
$wingetSetupIndex = $ciSource.IndexOf('name: Install current WinGet validation client', [StringComparison]::Ordinal)
$officialWingetIndex = $ciSource.IndexOf('name: Official WinGet manifest validation', [StringComparison]::Ordinal)
if ($toolchainIndex -lt 0 -or $packagingPolicyIndex -lt 0 -or $wingetSetupIndex -lt 0 -or $officialWingetIndex -lt 0 -or
    $packagingPolicyIndex -gt $wingetSetupIndex -or $wingetSetupIndex -gt $officialWingetIndex -or
    $officialWingetIndex -gt $toolchainIndex) {
    throw 'Packaging policy and official WinGet validation must fail fast before Rust toolchain setup.'
}
$releaseWorkflowSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\release.yml')
$releaseWindowsJobIndex = $releaseWorkflowSource.IndexOf("  windows:`n", [StringComparison]::Ordinal)
$releaseWindowsTimeoutIndex = $releaseWorkflowSource.IndexOf('    timeout-minutes: 90', [StringComparison]::Ordinal)
$releaseSignJobIndex = $releaseWorkflowSource.IndexOf("  sign:`n", [StringComparison]::Ordinal)
if ($releaseWindowsJobIndex -lt 0 -or $releaseWindowsTimeoutIndex -lt 0 -or $releaseSignJobIndex -lt 0 -or
    $releaseWindowsTimeoutIndex -lt $releaseWindowsJobIndex -or $releaseWindowsTimeoutIndex -gt $releaseSignJobIndex) {
    throw 'Release Windows packaging jobs do not have the required 90-minute hard timeout.'
}
Assert-WorkflowJobTimeout -Source $releaseWorkflowSource -Job 'sbom' -Minutes 20 -Workflow 'Release'
Assert-WorkflowJobTimeout -Source $releaseWorkflowSource -Job 'publish' -Minutes 30 -Workflow 'Release'
Assert-WorkflowJobTimeout -Source $releaseWorkflowSource -Job 'publish-stage' -Minutes 30 -Workflow 'Release'
$reproducibilityWorkflowSource = Get-Content -Raw -LiteralPath $reproducibilityWorkflow
foreach ($requiredReproducibilityToken in @(
    'cancel-in-progress: true',
    'timeout-minutes: 120',
    'fail-fast: false',
    'architecture: [x64, arm64]',
    'if: always()',
    'if-no-files-found: error',
    'retention-days: 30'
)) {
    if ($reproducibilityWorkflowSource -notmatch [Regex]::Escape($requiredReproducibilityToken)) {
        throw "The reproducibility workflow omits required bounded-execution token: $requiredReproducibilityToken"
    }
}
foreach ($requiredWingetToken in @(
    './packaging/winget/Generate-Manifests.ps1',
    './packaging/winget/Test-Manifests.ps1',
    'target/winget/manifests/z/ZiCode/ZiFile/$version',
    '-X64InstallerPath $x64.FullName',
    '-Arm64InstallerPath $arm64.FullName'
)) {
    if ($releaseWorkflowSource -notmatch [Regex]::Escape($requiredWingetToken)) {
        throw "Release does not enforce the verified WinGet candidate token: $requiredWingetToken"
    }
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
    'rar5-default-truncated-half',
    'truncated-archive',
    'rar5-default-corrupt-middle',
    'flip-middle-byte',
    'corrupt-payload',
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
$cabInteroperabilitySource = Get-Content -Raw -LiteralPath $cabInteroperability
foreach ($requiredCabToken in @(
    'makecab.exe',
    'expand.exe',
    "type = 'MSZIP'",
    "type = 'LZX'",
    'target\cab-interoperability.json',
    'matched = $true'
)) {
    if ($cabInteroperabilitySource -notmatch [Regex]::Escape($requiredCabToken)) {
        throw "The CAB interoperability gate omits required token: $requiredCabToken"
    }
}
foreach ($requiredCabCiToken in @(
    'cab-windows.ps1 -SkipBuild',
    'Upload CAB interoperability evidence',
    'target/cab-interoperability.json'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredCabCiToken)) {
        throw "CI does not publish the CAB interoperability gate or evidence: $requiredCabCiToken"
    }
}
$windowsToolsSource = Get-Content -Raw -LiteralPath $windowsTools
foreach ($requiredWindowsToolsToken in @(
    'tar.exe -c --lzma -f',
    'reference.tar.lzma',
    '--format tar-lzma',
    'tar.exe -x --lzma -f',
    'ZIP, tar.gz, tar.lzma and 7z'
)) {
    if ($windowsToolsSource -notmatch [Regex]::Escape($requiredWindowsToolsToken)) {
        throw "The Windows reference interoperability gate omits required token: $requiredWindowsToolsToken"
    }
}
foreach ($requiredWindowsToolsCiToken in @(
    'windows-tools.ps1 -SkipBuild',
    'Upload Windows reference-tool interoperability evidence',
    'target/windows-tools-interoperability.json'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredWindowsToolsCiToken)) {
        throw "CI does not publish the Windows reference interoperability evidence: $requiredWindowsToolsCiToken"
    }
}
foreach ($requiredContractPolicyToken in @(
    'formats',
    'tar-lzma',
    'runtime_error_exit_code = 1',
    'syntax_error_exit_code = 2',
    'bilingual_contract_docs_checked'
)) {
    $contractPolicySource = Get-Content -Raw -LiteralPath $contractPolicy
    if ($contractPolicySource -notmatch [Regex]::Escape($requiredContractPolicyToken)) {
        throw "The CLI contract smoke omits required token: $requiredContractPolicyToken"
    }
}
foreach ($requiredContractCiToken in @(
    'CLI contract smoke test',
    'contract-policy.ps1 -SkipBuild'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredContractCiToken)) {
        throw "CI does not execute the CLI contract smoke: $requiredContractCiToken"
    }
}
$zipMethodCorpusSource = Get-Content -Raw -LiteralPath $zipMethodCorpus
foreach ($requiredZipMethodToken in @(
    "name = 'deflate64'",
    "name = 'bzip2'",
    "name = 'lzma'",
    "name = 'xz'",
    "name = 'ppmd'",
    "encryption = 'AES256'",
    "encryption = 'ZipCrypto'",
    'Get-SevenZipMethods',
    'Assert-FixtureMatches',
    'target\zip-method-corpus.json'
)) {
    if ($zipMethodCorpusSource -notmatch [Regex]::Escape($requiredZipMethodToken)) {
        throw "The ZIP method corpus gate omits required token: $requiredZipMethodToken"
    }
}
foreach ($requiredZipMethodCiToken in @(
    'zip-method-corpus.ps1 -SkipBuild',
    'Upload ZIP method interoperability evidence',
    'target/zip-method-corpus.json'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredZipMethodCiToken)) {
        throw "CI does not publish the ZIP method corpus gate or evidence: $requiredZipMethodCiToken"
    }
}
$workspaceManifestSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot 'Cargo.toml')
if ($workspaceManifestSource -notmatch 'zip\s*=\s*\{[^}]*features\s*=\s*\["legacy-zip"\]') {
    throw 'The workspace ZIP backend does not enable legacy-zip decoding.'
}
$zipLegacyCorpusSource = Get-Content -Raw -LiteralPath $zipLegacyCorpus
foreach ($requiredZipLegacyToken in @(
    '771dfc534d2614158af5497ea3dff4d4208d7db1',
    "name = 'shrink'",
    "name = 'reduce'",
    "name = 'implode'",
    'GetByteArrayAsync',
    'Get-Sha256',
    'Assert-SingleGoldenFile',
    'target\zip-legacy-corpus.json'
)) {
    if ($zipLegacyCorpusSource -notmatch [Regex]::Escape($requiredZipLegacyToken)) {
        throw "The ZIP legacy corpus gate omits required token: $requiredZipLegacyToken"
    }
}
foreach ($requiredZipLegacyCiToken in @(
    'zip-legacy-corpus.ps1 -SkipBuild',
    'Upload ZIP legacy interoperability evidence',
    'target/zip-legacy-corpus.json'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredZipLegacyCiToken)) {
        throw "CI does not publish the ZIP legacy corpus gate or evidence: $requiredZipLegacyCiToken"
    }
}
$zipZstdCorpusSource = Get-Content -Raw -LiteralPath $zipZstdCorpus
foreach ($requiredZipZstdToken in @(
    'ee079b86fbd3817c53fe245bea4effaaaf1d97f7',
    'test_read_format_zip_zstd.zipx.uu',
    'test_read_format_zip_zstd_multi.zipx.uu',
    'ConvertFrom-UuBytes',
    'MaximumDecodedBytes = 16777216',
    'Get-Sha256Bytes',
    'Assert-ExtractedFiles',
    'target\zip-zstd-corpus.json'
)) {
    if ($zipZstdCorpusSource -notmatch [Regex]::Escape($requiredZipZstdToken)) {
        throw "The ZIP Zstandard corpus gate omits required token: $requiredZipZstdToken"
    }
}
foreach ($requiredZipZstdCiToken in @(
    'zip-zstd-corpus.ps1 -SkipBuild',
    'Upload ZIP Zstandard interoperability evidence',
    'target/zip-zstd-corpus.json'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredZipZstdCiToken)) {
        throw "CI does not publish the ZIP Zstandard corpus gate or evidence: $requiredZipZstdCiToken"
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
$wackReadinessSource = Get-Content -Raw -LiteralPath $wackReadiness
foreach ($requiredWackIdentityToken in @(
    '[Parameter(Mandatory)][string]$ExpectedIdentityName',
    '[Parameter(Mandatory)][string]$ExpectedPublisher',
    '[Parameter(Mandatory)][string]$ExpectedPublisherDisplayName',
    "Add-ReadinessIssue 'identity_mismatch'",
    "Add-ReadinessIssue 'publisher_mismatch'",
    "Add-ReadinessIssue 'publisher_display_name_mismatch'"
)) {
    if ($wackReadinessSource -notmatch [Regex]::Escape($requiredWackIdentityToken)) {
        throw "WACK readiness does not require the exact Partner Center tuple: $requiredWackIdentityToken"
    }
}
$docsPagesSource = Get-Content -Raw -LiteralPath (Join-Path $repoRoot '.github\workflows\docs-pages.yml')
Assert-WorkflowJobTimeout -Source $docsPagesSource -Job 'build' -Minutes 20 -Workflow 'Docs Pages'
Assert-WorkflowJobTimeout -Source $docsPagesSource -Job 'deploy' -Minutes 15 -Workflow 'Docs Pages'
foreach ($requiredPrivacyToken in @(
    'Validate generated Store privacy routes',
    './packaging/store/Test-PublicPrivacy.ps1 -DocumentationOutput ./docs/dist'
)) {
    if ($ciSource -notmatch [Regex]::Escape($requiredPrivacyToken) -or
        $docsPagesSource -notmatch [Regex]::Escape($requiredPrivacyToken)) {
        throw "CI and Pages must validate generated Store privacy routes: $requiredPrivacyToken"
    }
}
foreach ($requiredLivePrivacyToken in @(
    'packaging/store/listing.*.json',
    'packaging/store/Test-PublicPrivacy.ps1',
    'Validate deployed Store privacy routes',
    './packaging/store/Test-PublicPrivacy.ps1 -Live'
)) {
    if ($docsPagesSource -notmatch [Regex]::Escape($requiredLivePrivacyToken)) {
        throw "Pages deployment does not validate the public Store privacy routes: $requiredLivePrivacyToken"
    }
}

[pscustomobject]@{
    schema_version = 1
    parser_checks = $scriptsToParse.Count
    multiresolution_icon_accepted = $true
    truncated_icon_directory_rejected = $true
    resource_less_executable_rejected = $true
    embedded_icon_package_audit_wired = $true
    missing_inputs_rejected = $true
    development_identity_rejected = $true
    unsigned_publisher_rejected = $true
    formal_inputs_accepted = $true
    stable_pfx_release_rejected = $true
    malformed_release_version_rejected = $true
    package_audit_wired = $true
    release_audit_staged = $true
    shell_extension_wired = $true
    shell_extract_command_wired = $true
    lifecycle_gate_wired = $true
    lifecycle_workflow_wired = $true
    lifecycle_signed_artifacts_wired = $true
    repair_helper_wired = $true
    rar_association_wired = $true
    rar_corpus_wired = $true
    cab_association_wired = $true
    zipx_association_wired = $true
    archive_alias_ingress_wired = $true
    cab_interoperability_wired = $true
    windows_tools_interoperability_wired = $true
    cli_contract_smoke_wired = $true
    zip_method_corpus_wired = $true
    zip_legacy_corpus_wired = $true
    zip_zstd_corpus_wired = $true
    wack_readiness_wired = $true
    wack_partner_center_tuple_required = $true
    version_consistency_wired = $true
    release_notes_wired = $true
    contributor_docs_wired = $true
    security_docs_wired = $true
    release_readiness_wired = $true
    stable_release_pending_rejected = $true
    complete_release_readiness_accepted = $true
    cloud_signing_inputs_accepted = $true
    unsupported_signing_provider_rejected = $true
    unsigned_release_artifacts_rejected = $true
    production_signing_workflow_wired = $true
    pfx_release_inputs_retired = $true
    signing_operations_docs_wired = $true
    least_privilege_release_permissions = $true
    signing_timeout_wired = $true
    signing_concurrency_wired = $true
    release_packaging_timeout_wired = $true
    workflow_job_timeouts_wired = $true
    reproducibility_execution_bounded = $true
    foreground_queue_ownership_wired = $true
    partner_center_identity_preflight_wired = $true
    partner_center_publisher_display_name_wired = $true
    partial_partner_center_identity_rejected = $true
    malformed_partner_center_identity_rejected = $true
    winget_community_path_generated = $true
    winget_local_hashes_verified = $true
    winget_invalid_release_url_rejected = $true
    winget_release_gate_wired = $true
    official_winget_ci_validation_wired = $true
    user_docs_wired = $true
    packaging_release_gates_fail_fast = $true
    public_privacy_routes_wired = $true
    msix_assets_validated = $true
    malformed_msix_asset_rejected = $true
    missing_manifest_asset_rejected = $true
    pinned_asset_drift_rejected = $true
    missing_qualified_msix_asset_rejected = $true
    high_dpi_msix_asset_matrix_locked = $true
    packaged_visual_asset_hashes_wired = $true
    store_listing_asset_validated = $true
    malformed_store_listing_assets_rejected = $true
} | ConvertTo-Json
