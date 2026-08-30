param(
    [Parameter(Mandatory)]
    [string]$Directory,
    [Parameter(Mandatory)]
    [ValidatePattern('^\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?$')]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$directoryPath = [IO.Path]::GetFullPath($Directory)
if (-not (Test-Path -LiteralPath $directoryPath -PathType Container)) {
    throw "Public release directory does not exist: $directoryPath"
}

$expectedBundle = "ZiFile-$Version.0-windows.msixbundle"
$expectedNames = @(
    'SHA256SUMS.txt',
    $expectedBundle,
    'zifile-windows-arm64.exe',
    'zifile-windows-x64.exe'
)
$files = @(Get-ChildItem -LiteralPath $directoryPath -File | Sort-Object Name)
$actualNames = @($files.Name)
if ($actualNames.Count -ne $expectedNames.Count -or
    (Compare-Object -ReferenceObject $expectedNames -DifferenceObject $actualNames)) {
    throw "Public release must contain exactly: $($expectedNames -join ', '). Actual: $($actualNames -join ', ')."
}

foreach ($file in $files) {
    if ($file.Length -le 0) {
        throw "Public release asset is empty: $($file.Name)"
    }
}

$checksumFile = Join-Path $directoryPath 'SHA256SUMS.txt'
$records = @(
    foreach ($line in @(Get-Content -LiteralPath $checksumFile -Encoding utf8)) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        if ($line -notmatch '^\s*(?<hash>[A-Fa-f0-9]{64})\s+(?:\*)?(?<name>.+?)\s*$') {
            throw "Malformed SHA256SUMS line: $line"
        }
        $name = $Matches['name'].Trim()
        if ($name.StartsWith('./', [StringComparison]::Ordinal)) {
            $name = $name.Substring(2)
        }
        if ($name -ne [IO.Path]::GetFileName($name)) {
            throw "SHA256SUMS contains a path instead of a release filename: $name"
        }
        [pscustomobject]@{
            name = $name
            hash = $Matches['hash'].ToUpperInvariant()
        }
    }
)
$payloadNames = @($expectedNames | Where-Object { $_ -ne 'SHA256SUMS.txt' })
$checksumNames = @($records | ForEach-Object name)
if ($records.Count -ne $payloadNames.Count -or
    @($checksumNames | Sort-Object -Unique).Count -ne $payloadNames.Count -or
    (Compare-Object -ReferenceObject $payloadNames -DifferenceObject $checksumNames)) {
    throw 'SHA256SUMS must contain exactly one entry for each user-facing installer and portable executable.'
}

foreach ($record in $records) {
    $asset = Join-Path $directoryPath $record.name
    $actualHash = (Get-FileHash -LiteralPath $asset -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($actualHash -cne $record.hash) {
        throw "SHA256SUMS hash mismatch for $($record.name)."
    }
}

[pscustomobject]@{
    schema_version = 1
    version = $Version
    user_facing_asset_count = $expectedNames.Count
    all_in_one_bundle = $expectedBundle
    standalone_executables = @('zifile-windows-x64.exe', 'zifile-windows-arm64.exe')
    forbidden_internal_assets_absent = $true
    checksums_verified = $true
} | ConvertTo-Json -Depth 4
