[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$SourceDirectory,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$SourceCommit,
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$')][string]$AppVersion,
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$WindowsBuild,
    [Parameter(Mandatory)][ValidateSet('light', 'dark')][string]$Theme,
    [Parameter(Mandatory)][ValidateRange(100, 500)][int]$ScalePercent,
    [Parameter(Mandatory)][DateTimeOffset]$CapturedAtUtc,
    [ValidateSet('signed-msix', 'store-signed-msix')][string]$CandidateKind = 'signed-msix',
    [string]$DestinationRoot = $PSScriptRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$sourceRoot = [IO.Path]::GetFullPath($SourceDirectory)
$destination = [IO.Path]::GetFullPath($DestinationRoot)
$manifestPath = Join-Path $destination 'screenshots.json'
$assetsPath = Join-Path $destination 'assets'
$validator = Join-Path $PSScriptRoot 'Test-Screenshots.ps1'

if (-not (Test-Path -LiteralPath $sourceRoot -PathType Container)) {
    throw "Screenshot source directory does not exist: $sourceRoot"
}
if (-not (Test-Path -LiteralPath $destination -PathType Container)) {
    throw "Store destination directory does not exist: $destination"
}
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Store destination is missing screenshots.json: $destination"
}
if (Test-Path -LiteralPath $assetsPath) {
    throw "Store screenshot assets already exist; refusing to overwrite: $assetsPath"
}
$current = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
if ($current.schema_version -ne 1 -or $current.status -cne 'draft') {
    throw 'Screenshot import requires an existing schema v1 draft manifest.'
}
if ($CapturedAtUtc.Offset -ne [TimeSpan]::Zero) {
    throw 'CapturedAtUtc must include the UTC +00:00 offset.'
}

$definitions = @(
    [pscustomobject]@{ locale = 'zh-CN'; order = 1; scenario = 'home'; name = '01-home.png'; caption = '打开、浏览和安全管理本地压缩包' },
    [pscustomobject]@{ locale = 'zh-CN'; order = 2; scenario = 'create'; name = '02-create.png'; caption = '创建 ZIP、7z、TAR 和常用压缩流' },
    [pscustomobject]@{ locale = 'zh-CN'; order = 3; scenario = 'browse'; name = '03-browse.png'; caption = '搜索、选择并校验压缩包内容' },
    [pscustomobject]@{ locale = 'zh-CN'; order = 4; scenario = 'extract'; name = '04-extract.png'; caption = '选择冲突策略并查看解压任务进度' },
    [pscustomobject]@{ locale = 'en-US'; order = 1; scenario = 'home'; name = '01-home.png'; caption = 'Open, browse, and safely manage local archives' },
    [pscustomobject]@{ locale = 'en-US'; order = 2; scenario = 'create'; name = '02-create.png'; caption = 'Create ZIP, 7z, TAR, and common compressed streams' },
    [pscustomobject]@{ locale = 'en-US'; order = 3; scenario = 'browse'; name = '03-browse.png'; caption = 'Search, select, and test archive contents' },
    [pscustomobject]@{ locale = 'en-US'; order = 4; scenario = 'extract'; name = '04-extract.png'; caption = 'Choose a conflict policy and monitor extraction progress' }
)

foreach ($definition in $definitions) {
    $source = Join-Path $sourceRoot (Join-Path $definition.locale $definition.name)
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Missing required screenshot: $source"
    }
}

$stagingName = ".screenshot-import-$([Guid]::NewGuid().ToString('N'))"
$staging = Join-Path $destination $stagingName
New-Item -ItemType Directory -Path $staging -ErrorAction Stop | Out-Null
$assetsMoved = $false
try {
    $localeSets = @()
    foreach ($locale in @('zh-CN', 'en-US')) {
        $shots = @()
        foreach ($definition in $definitions | Where-Object locale -CEQ $locale) {
            $relative = "assets/$locale/desktop/$($definition.name)"
            $staged = Join-Path $staging ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
            New-Item -ItemType Directory -Path (Split-Path -Parent $staged) -Force | Out-Null
            $source = Join-Path $sourceRoot (Join-Path $locale $definition.name)
            Copy-Item -LiteralPath $source -Destination $staged -ErrorAction Stop
            $shots += [ordered]@{
                order = $definition.order
                scenario = $definition.scenario
                path = $relative
                caption = $definition.caption
                sha256 = (Get-FileHash -LiteralPath $staged -Algorithm SHA256).Hash
            }
        }
        $localeSets += [ordered]@{ locale = $locale; screenshots = $shots }
    }

    $manifest = [ordered]@{
        schema_version = 1
        status = 'complete'
        source_commit = $SourceCommit
        requirements_source = 'https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/screenshots-and-images'
        capture = [ordered]@{
            app_version = $AppVersion
            windows_build = $WindowsBuild
            theme = $Theme
            scale_percent = $ScalePercent
            captured_at_utc = $CapturedAtUtc.ToString('yyyy-MM-ddTHH:mm:ssZ')
            candidate_kind = $CandidateKind
        }
        locales = $localeSets
    }
    $stagedManifest = Join-Path $staging 'screenshots.json'
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $stagedManifest -Encoding utf8
    $validation = & $validator -ManifestPath $stagedManifest -RequireComplete | ConvertFrom-Json
    if (-not $validation.complete -or $validation.screenshots -ne 8) {
        throw 'Staged Store screenshots did not produce a complete eight-image validation.'
    }

    Move-Item -LiteralPath (Join-Path $staging 'assets') -Destination $assetsPath -ErrorAction Stop
    $assetsMoved = $true
    try {
        Move-Item -LiteralPath $stagedManifest -Destination $manifestPath -Force -ErrorAction Stop
    }
    catch {
        Move-Item -LiteralPath $assetsPath -Destination (Join-Path $staging 'assets') -ErrorAction SilentlyContinue
        $assetsMoved = $false
        throw
    }

    [pscustomobject]@{
        schema_version = 1
        imported = $true
        screenshots = 8
        source_commit = $SourceCommit
        destination = $destination
    } | ConvertTo-Json
}
finally {
    if (-not $assetsMoved -and (Test-Path -LiteralPath $staging)) {
        Write-Warning "Validated screenshot staging was retained for diagnosis: $staging"
    }
    elseif (Test-Path -LiteralPath $staging) {
        Remove-Item -LiteralPath $staging -Force
    }
}
