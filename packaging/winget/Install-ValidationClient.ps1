param(
    [string]$ModuleVersion = '1.29.280',
    [string]$ClientVersion = '1.29.280'
)

$ErrorActionPreference = 'Stop'

Install-Module `
    -Name Microsoft.WinGet.Client `
    -RequiredVersion $ModuleVersion `
    -Repository PSGallery `
    -Scope CurrentUser `
    -Force `
    -AllowClobber
Import-Module Microsoft.WinGet.Client -RequiredVersion $ModuleVersion -Force
Repair-WinGetPackageManager -Version $ClientVersion -Force

$winget = Get-Command winget.exe -ErrorAction Stop
$versionOutput = (& $winget.Source --version | Out-String).Trim()
if ($versionOutput -notmatch '^v(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)$') {
    throw "The repaired WinGet client returned an unexpected version: $versionOutput"
}
$actual = [Version]::new(
    [int]$Matches.major,
    [int]$Matches.minor,
    [int]$Matches.patch
)
$minimum = [Version]::new(1, 29, 280)
if ($actual -lt $minimum) {
    throw "WinGet $versionOutput is older than the required validation client v$minimum."
}

[pscustomobject]@{
    schema_version = 1
    module_version = $ModuleVersion
    requested_client_version = $ClientVersion
    actual_client_version = $versionOutput
    current_stable_client_pinned = $true
} | ConvertTo-Json
