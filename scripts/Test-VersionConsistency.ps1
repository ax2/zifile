param(
    [string]$ExpectedVersion,
    [string]$RepositoryRoot = (Join-Path $PSScriptRoot '..')
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
$cargoPath = Join-Path $repoRoot 'Cargo.toml'
$lockPath = Join-Path $repoRoot 'Cargo.lock'
$docsPackagePath = Join-Path $repoRoot 'docs\package.json'

$cargo = Get-Content -Raw -LiteralPath $cargoPath
$workspaceSection = [Regex]::Match(
    $cargo,
    '(?ms)^\[workspace\.package\]\s*\r?\n(?<body>.*?)(?=^\[|\z)'
)
if (-not $workspaceSection.Success) {
    throw 'Cargo.toml does not contain [workspace.package].'
}
$workspaceVersion = [Regex]::Match(
    $workspaceSection.Groups['body'].Value,
    '(?m)^version\s*=\s*"(?<version>[^"]+)"\s*$'
)
if (-not $workspaceVersion.Success) {
    throw 'Cargo.toml [workspace.package] does not declare version.'
}
$version = $workspaceVersion.Groups['version'].Value
$versionPattern = '^(?<major>0|[1-9]\d*)\.(?<minor>0|[1-9]\d*)\.(?<patch>0|[1-9]\d*)(?:-(?<channel>alpha|beta|rc)\.(?<revision>0|[1-9]\d*))?$'
if ($version -notmatch $versionPattern) {
    throw "Workspace version must be stable SemVer or alpha/beta/rc with a numeric revision: $version"
}
$major = [int]$Matches['major']
$minor = [int]$Matches['minor']
$patch = [int]$Matches['patch']
$revision = if ($Matches['revision']) { [int]$Matches['revision'] } else { 0 }
foreach ($component in @($major, $minor, $patch, $revision)) {
    if ($component -gt 65535) {
        throw "Version component exceeds the MSIX limit of 65535: $version"
    }
}

if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion)) {
    $normalizedExpected = if ($ExpectedVersion.StartsWith('v')) {
        $ExpectedVersion.Substring(1)
    }
    else {
        $ExpectedVersion
    }
    if ($normalizedExpected -ne $version) {
        throw "Release version '$normalizedExpected' does not match workspace version '$version'."
    }
}

$docsVersion = (Get-Content -Raw -LiteralPath $docsPackagePath | ConvertFrom-Json).version
if ($docsVersion -ne $version) {
    throw "docs/package.json version '$docsVersion' does not match workspace version '$version'."
}

foreach ($dependency in @('zifile-core', 'zifile-worker-protocol')) {
    $dependencyPattern = '(?m)^' + [Regex]::Escape($dependency) +
        '\s*=\s*\{[^\r\n]*version\s*=\s*"(?<version>[^"]+)"'
    $pin = [Regex]::Match(
        $cargo,
        $dependencyPattern
    )
    if (-not $pin.Success) {
        throw "Cargo.toml does not pin the internal dependency version for $dependency."
    }
    if ($pin.Groups['version'].Value -ne $version) {
        throw "Internal dependency $dependency is pinned to '$($pin.Groups['version'].Value)' instead of '$version'."
    }
}

$lock = Get-Content -Raw -LiteralPath $lockPath
$workspacePackages = @(
    'zifile-cli',
    'zifile-core',
    'zifile-desktop',
    'zifile-shell',
    'zifile-worker',
    'zifile-worker-protocol'
)
foreach ($package in $workspacePackages) {
    $packagePattern = '(?ms)^\[\[package\]\]\s*\r?\n' +
        '(?:(?!^\[\[package\]\]).)*?^name\s*=\s*"' +
        [Regex]::Escape($package) +
        '"\s*\r?\n^version\s*=\s*"(?<version>[^"]+)"'
    $block = [Regex]::Match(
        $lock,
        $packagePattern
    )
    if (-not $block.Success) {
        throw "Cargo.lock does not contain workspace package $package."
    }
    if ($block.Groups['version'].Value -ne $version) {
        throw "Cargo.lock package $package is '$($block.Groups['version'].Value)' instead of '$version'."
    }
}

[ordered]@{
    schema_version = 1
    version = $version
    tag = "v$version"
    msix_version = "$major.$minor.$patch.$revision"
    docs_version = $docsVersion
    workspace_packages = $workspacePackages.Count
    internal_dependency_pins = 2
    consistent = $true
} | ConvertTo-Json
