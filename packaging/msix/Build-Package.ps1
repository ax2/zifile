param(
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
    [string]$Version = '0.1.0.0',
    [ValidateSet('x64', 'arm64')]
    [string]$Architecture = 'x64',
    [string]$IdentityName = 'ZiCode.ZiFile.Dev',
    [string]$Publisher = 'CN=ZiCode Development, OID.2.25.311729368913984317654407730594956997722=1',
    [string]$CertificatePath,
    [securestring]$CertificatePassword,
    [switch]$AccessibleUi
)

$ErrorActionPreference = 'Stop'
$unsignedPublisherOid = 'OID.2.25.311729368913984317654407730594956997722=1'
$unsignedPublisherPattern = "(?i)(^|,\s*)$([Regex]::Escape($unsignedPublisherOid))($|,\s*)"
if ($CertificatePath -and $Publisher -match $unsignedPublisherPattern) {
    throw 'Signed packages cannot use the Windows unsigned publisher namespace. Pass the certificate subject as Publisher.'
}
if (-not $CertificatePath -and $IdentityName.EndsWith('.Dev') -and $Publisher -notmatch $unsignedPublisherPattern) {
    throw "Unsigned development packages require the Microsoft unsigned namespace identifier $unsignedPublisherOid in Publisher."
}
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$targetRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'target\package'))
$variantSuffix = if ($AccessibleUi) { '-accessible' } else { '' }
$stageRoot = [System.IO.Path]::GetFullPath((Join-Path $targetRoot "msix-$Architecture$variantSuffix"))
$distRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'dist'))
$runnableRoot = Join-Path $distRoot "ZiFile-$Version-windows-$Architecture$variantSuffix"
$msixPath = Join-Path $distRoot "ZiFile-$Version-windows-$Architecture$variantSuffix.msix"
$auditPath = Join-Path $distRoot "ZiFile-$Version-windows-$Architecture$variantSuffix.audit.json"

foreach ($path in @($stageRoot, $runnableRoot)) {
    if (-not $path.StartsWith($repoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to stage outside the repository: $path"
    }
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force
    }
}

$iconPath = Join-Path $PSScriptRoot 'Assets\ZiFile.ico'
if (-not (Test-Path -LiteralPath $iconPath)) {
    ./packaging/msix/Generate-Assets.ps1
}

$rustTarget = if ($Architecture -eq 'arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$compilerArchitecture = if ($Architecture -eq 'arm64') { 'arm64' } else { 'x64' }
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    $developerCommandCandidates = [System.Collections.Generic.List[string]]::new()
    $vsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vsWhere -PathType Leaf) {
        $installations = @(& $vsWhere -all -products * -property installationPath)
        foreach ($installation in $installations) {
            if (-not [string]::IsNullOrWhiteSpace($installation)) {
                $candidate = Join-Path $installation 'Common7\Tools\VsDevCmd.bat'
                if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                    $developerCommandCandidates.Add($candidate)
                }
            }
        }
    }
    foreach ($visualStudioRoot in @(
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio'),
        (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio')
    )) {
        if (Test-Path -LiteralPath $visualStudioRoot -PathType Container) {
            Get-ChildItem -LiteralPath $visualStudioRoot -Recurse -Filter VsDevCmd.bat -ErrorAction SilentlyContinue |
                ForEach-Object { $developerCommandCandidates.Add($_.FullName) }
        }
    }
    $developerCommand = $developerCommandCandidates |
        Sort-Object @{ Expression = { if ($_ -match '\\2022\\') { 0 } else { 1 } } }, @{ Expression = { $_ } } |
        Select-Object -Unique -First 1
    if (-not $developerCommand) {
        throw 'MSVC compiler is unavailable and VsDevCmd.bat could not be found.'
    }
    $hostArchitecture = if (
        [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq
        [Runtime.InteropServices.Architecture]::Arm64
    ) { 'arm64' } else { 'x64' }
    $environmentOutput = & $env:ComSpec /d /s /c (
        '"{0}" -no_logo -arch={1} -host_arch={2} && set' -f
        $developerCommand, $compilerArchitecture, $hostArchitecture
    )
    if ($LASTEXITCODE -ne 0) {
        throw "Visual Studio developer environment initialization failed for $compilerArchitecture."
    }
    foreach ($line in $environmentOutput) {
        if ($line -match '^([^=][^=]*)=(.*)$') {
            [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process')
        }
    }
    if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
        throw "Visual Studio developer environment did not expose cl.exe for $compilerArchitecture."
    }
}
if ($Architecture -eq 'arm64' -and $env:VCToolsInstallDir) {
    $hostFolder = if (
        [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq
        [Runtime.InteropServices.Architecture]::Arm64
    ) { 'HostARM64' } else { 'HostX64' }
    $arm64Compiler = Join-Path $env:VCToolsInstallDir "bin\$hostFolder\arm64\cl.exe"
    $arm64Runtime = Join-Path $env:VCToolsInstallDir 'lib\arm64\msvcrt.lib'
    if (
        -not (Test-Path -LiteralPath $arm64Compiler -PathType Leaf) -or
        -not (Test-Path -LiteralPath $arm64Runtime -PathType Leaf)
    ) {
        throw 'The MSVC ARM64 build tools are incomplete. Install the Visual Studio MSVC ARM64/ARM64EC build tools component.'
    }
}
rustup target add $rustTarget
if ($AccessibleUi) {
    cargo build --workspace --release --locked --target $rustTarget --all-features
} else {
    cargo build --workspace --release --locked --target $rustTarget
}
if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }

$binaryRoot = Join-Path $repoRoot "target\$rustTarget\release"
$desktopBinary = if ($AccessibleUi) { 'zifile-desktop-accessible.exe' } else { 'zifile-desktop.exe' }
New-Item -ItemType Directory -Path (Join-Path $stageRoot 'ZiFile') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $stageRoot 'Assets') -Force | Out-Null
New-Item -ItemType Directory -Path $runnableRoot -Force | Out-Null

Copy-Item -LiteralPath (Join-Path $binaryRoot $desktopBinary) -Destination (Join-Path $stageRoot 'ZiFile\zifile-desktop.exe')
Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile.exe') -Destination (Join-Path $stageRoot 'ZiFile\zifile.exe')
Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile-worker.exe') -Destination (Join-Path $stageRoot 'ZiFile\zifile-worker.exe')
Copy-Item -Path (Join-Path $PSScriptRoot 'Assets\*') -Destination (Join-Path $stageRoot 'Assets')
Copy-Item -LiteralPath (Join-Path $binaryRoot $desktopBinary) -Destination (Join-Path $runnableRoot 'zifile-desktop.exe')
Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile.exe') -Destination $runnableRoot
Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile-worker.exe') -Destination $runnableRoot
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $runnableRoot
Copy-Item -LiteralPath (Join-Path $repoRoot 'README.md') -Destination $runnableRoot

[xml]$manifest = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'AppxManifest.xml')
$identity = $manifest.Package.Identity
$identity.Name = $IdentityName
$identity.Publisher = $Publisher
$identity.Version = $Version
$identity.ProcessorArchitecture = $Architecture
$unsignedDevelopmentPackage = -not $CertificatePath -and $IdentityName.EndsWith('.Dev')
if ($unsignedDevelopmentPackage) {
    $manifest.Package.Dependencies.TargetDeviceFamily.MinVersion = '10.0.26100.0'
}
$manifest.Save((Join-Path $stageRoot 'AppxManifest.xml'))

$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$toolArchitecture = if (
    [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq
    [Runtime.InteropServices.Architecture]::Arm64
) { 'arm64' } else { 'x64' }
$makeAppx = Get-ChildItem -LiteralPath $sdkRoot -Recurse -Filter MakeAppx.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "\\$toolArchitecture\\MakeAppx\.exe$" } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $makeAppx) { throw 'MakeAppx.exe was not found. Install the Windows SDK packaging tools.' }

New-Item -ItemType Directory -Path $distRoot -Force | Out-Null
if (Test-Path -LiteralPath $msixPath) { Remove-Item -LiteralPath $msixPath -Force }
if (Test-Path -LiteralPath $auditPath) { Remove-Item -LiteralPath $auditPath -Force }
& $makeAppx.FullName pack /d $stageRoot /p $msixPath /o
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed.' }

if ($CertificatePath) {
    $signTool = Get-ChildItem -LiteralPath $sdkRoot -Recurse -Filter SignTool.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "\\$toolArchitecture\\SignTool\.exe$" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $signTool) { throw 'SignTool.exe was not found.' }
    if (-not $CertificatePassword) { throw 'CertificatePassword is required when CertificatePath is supplied.' }
    $password = [System.Net.NetworkCredential]::new('', $CertificatePassword).Password
    & $signTool.FullName sign /fd SHA256 /a /f $CertificatePath /p $password $msixPath
    if ($LASTEXITCODE -ne 0) { throw 'MSIX signing failed.' }
}

$auditArguments = @{
    PackagePath = $msixPath
    Architecture = $Architecture
    ExpectedVersion = $Version
    ExpectedIdentityName = $IdentityName
    ExpectedPublisher = $Publisher
    ExpectedMinimumVersion = if ($unsignedDevelopmentPackage) { '10.0.26100.0' } else { '10.0.19041.0' }
    EvidencePath = $auditPath
    RequireSignature = [bool]$CertificatePath
}
& (Join-Path $PSScriptRoot 'Test-Package.ps1') @auditArguments
if ($LASTEXITCODE -ne 0) { throw 'MSIX package audit failed.' }

Get-FileHash -LiteralPath $msixPath -Algorithm SHA256 |
    ForEach-Object { "{0}  {1}" -f $_.Hash.ToLowerInvariant(), (Split-Path $_.Path -Leaf) } |
    Set-Content -LiteralPath (Join-Path $distRoot "ZiFile-$Version-windows-$Architecture$variantSuffix.sha256")

Write-Host "Runnable directory: $runnableRoot"
Write-Host "MSIX package: $msixPath"
