param(
    [Parameter(Mandatory)]
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
    [string]$Version,
    [Parameter(Mandatory)]
    [string]$InputDirectory,
    [Parameter(Mandatory)]
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$inputRoot = [IO.Path]::GetFullPath($InputDirectory)
$bundlePath = [IO.Path]::GetFullPath($OutputPath)
if (-not (Test-Path -LiteralPath $inputRoot -PathType Container)) {
    throw "Bundle input directory does not exist: $inputRoot"
}

$packages = @(
    Join-Path $inputRoot "ZiFile-$Version-windows-x64.msix"
    Join-Path $inputRoot "ZiFile-$Version-windows-arm64.msix"
)
foreach ($package in $packages) {
    if (-not (Test-Path -LiteralPath $package -PathType Leaf)) {
        throw "Required architecture package is missing: $package"
    }
}

$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$toolArchitecture = if (
    [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq
    [Runtime.InteropServices.Architecture]::Arm64
) { 'arm64' } else { 'x64' }
$makeAppx = Get-ChildItem -LiteralPath $sdkRoot -Recurse -Filter MakeAppx.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "\\$toolArchitecture\\MakeAppx\.exe$" } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $makeAppx) {
    throw 'MakeAppx.exe was not found. Install the Windows SDK packaging tools.'
}

$bundleDirectory = Split-Path -Parent $bundlePath
New-Item -ItemType Directory -Path $bundleDirectory -Force | Out-Null
if (Test-Path -LiteralPath $bundlePath) {
    Remove-Item -LiteralPath $bundlePath -Force
}

$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) "zifile-msixbundle-$([Guid]::NewGuid().ToString('N'))"
try {
    New-Item -ItemType Directory -Path $temporaryRoot -Force | Out-Null
    $packages | Copy-Item -Destination $temporaryRoot -Force
    & $makeAppx.FullName bundle /d $temporaryRoot /p $bundlePath /o
    if ($LASTEXITCODE -ne 0) {
        throw 'MakeAppx bundle failed.'
    }
    if (-not (Test-Path -LiteralPath $bundlePath -PathType Leaf)) {
        throw "MakeAppx did not create the bundle: $bundlePath"
    }
} finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}

$hash = (Get-FileHash -LiteralPath $bundlePath -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "MSIX bundle: $bundlePath"
Write-Host "SHA-256: $hash"
