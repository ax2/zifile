param(
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
    [string]$Version = '0.1.0.0',
    [ValidateSet('x64', 'arm64')]
    [string]$Architecture = 'x64',
    [string]$IdentityName = 'ZiCode.ZiFile.Dev',
    [string]$Publisher = 'CN=ZiCode',
    [string]$CertificatePath,
    [securestring]$CertificatePassword
)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$targetRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'target\package'))
$stageRoot = [System.IO.Path]::GetFullPath((Join-Path $targetRoot "msix-$Architecture"))
$distRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'dist'))
$runnableRoot = Join-Path $distRoot "ZiFile-$Version-windows-$Architecture"
$msixPath = Join-Path $distRoot "ZiFile-$Version-windows-$Architecture.msix"

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
rustup target add $rustTarget
cargo build --workspace --release --locked --target $rustTarget
if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }

$binaryRoot = Join-Path $repoRoot "target\$rustTarget\release"
New-Item -ItemType Directory -Path (Join-Path $stageRoot 'ZiFile') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $stageRoot 'Assets') -Force | Out-Null
New-Item -ItemType Directory -Path $runnableRoot -Force | Out-Null

Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile-desktop.exe') -Destination (Join-Path $stageRoot 'ZiFile\zifile-desktop.exe')
Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile.exe') -Destination (Join-Path $stageRoot 'ZiFile\zifile.exe')
Copy-Item -Path (Join-Path $PSScriptRoot 'Assets\*') -Destination (Join-Path $stageRoot 'Assets')
Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile-desktop.exe') -Destination $runnableRoot
Copy-Item -LiteralPath (Join-Path $binaryRoot 'zifile.exe') -Destination $runnableRoot
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $runnableRoot
Copy-Item -LiteralPath (Join-Path $repoRoot 'README.md') -Destination $runnableRoot

[xml]$manifest = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'AppxManifest.xml')
$identity = $manifest.Package.Identity
$identity.Name = $IdentityName
$identity.Publisher = $Publisher
$identity.Version = $Version
$identity.ProcessorArchitecture = $Architecture
$manifest.Save((Join-Path $stageRoot 'AppxManifest.xml'))

$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$makeAppx = Get-ChildItem -LiteralPath $sdkRoot -Recurse -Filter MakeAppx.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "\\$Architecture\\MakeAppx\.exe$" } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $makeAppx) { throw 'MakeAppx.exe was not found. Install the Windows SDK packaging tools.' }

New-Item -ItemType Directory -Path $distRoot -Force | Out-Null
if (Test-Path -LiteralPath $msixPath) { Remove-Item -LiteralPath $msixPath -Force }
& $makeAppx.FullName pack /d $stageRoot /p $msixPath /o
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed.' }

if ($CertificatePath) {
    $signTool = Get-ChildItem -LiteralPath $sdkRoot -Recurse -Filter SignTool.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "\\$Architecture\\SignTool\.exe$" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $signTool) { throw 'SignTool.exe was not found.' }
    if (-not $CertificatePassword) { throw 'CertificatePassword is required when CertificatePath is supplied.' }
    $password = [System.Net.NetworkCredential]::new('', $CertificatePassword).Password
    & $signTool.FullName sign /fd SHA256 /a /f $CertificatePath /p $password $msixPath
    if ($LASTEXITCODE -ne 0) { throw 'MSIX signing failed.' }
}

Get-FileHash -LiteralPath $msixPath -Algorithm SHA256 |
    ForEach-Object { "{0}  {1}" -f $_.Hash.ToLowerInvariant(), (Split-Path $_.Path -Leaf) } |
    Set-Content -LiteralPath (Join-Path $distRoot "ZiFile-$Version-windows-$Architecture.sha256")

Write-Host "Runnable directory: $runnableRoot"
Write-Host "MSIX package: $msixPath"
