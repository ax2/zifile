param(
    [string]$ModuleVersion = '1.29.280',
    [string]$ClientVersion = '1.29.280',
    [ValidateRange(1, 5)]
    [int]$RepairAttempts = 3
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
$repairFailure = $null
for ($attempt = 1; $attempt -le $RepairAttempts; $attempt++) {
    try {
        Repair-WinGetPackageManager -Version $ClientVersion -Force
        $repairFailure = $null
        break
    }
    catch {
        $repairFailure = $_
        if ($attempt -lt $RepairAttempts) {
            $delaySeconds = 5 * $attempt
            Write-Warning "WinGet repair attempt $attempt/$RepairAttempts failed; retrying in $delaySeconds seconds."
            Start-Sleep -Seconds $delaySeconds
        }
    }
}
if ($null -ne $repairFailure) {
    throw $repairFailure
}

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
    repair_attempts = $RepairAttempts
    actual_client_version = $versionOutput
    current_stable_client_pinned = $true
} | ConvertTo-Json
