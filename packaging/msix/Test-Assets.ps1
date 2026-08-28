[CmdletBinding()]
param(
    [string]$AssetsDirectory = (Join-Path $PSScriptRoot 'Assets'),
    [string]$AssetCatalogPath = (Join-Path $PSScriptRoot 'assets.json'),
    [string]$ManifestPath = (Join-Path $PSScriptRoot 'AppxManifest.xml'),
    [string]$GeneratorPath = (Join-Path $PSScriptRoot 'Generate-Assets.ps1'),
    [switch]$VerifyGenerator
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$assets = [IO.Path]::GetFullPath($AssetsDirectory)
$catalog = [IO.Path]::GetFullPath($AssetCatalogPath)
$manifest = [IO.Path]::GetFullPath($ManifestPath)
$generator = [IO.Path]::GetFullPath($GeneratorPath)
if (-not (Test-Path -LiteralPath $assets -PathType Container)) {
    throw "MSIX asset directory does not exist: $assets"
}
foreach ($file in @($catalog, $manifest, $generator)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required MSIX asset input does not exist: $file"
    }
}

Add-Type -AssemblyName System.Drawing

function Get-ImagePixelHash {
    param([Parameter(Mandatory)][byte[]]$Bytes)

    $stream = [IO.MemoryStream]::new($Bytes, $false)
    $bitmap = [Drawing.Bitmap]::new($stream)
    try {
        $pixels = [byte[]]::new($bitmap.Width * $bitmap.Height * 4)
        $offset = 0
        for ($y = 0; $y -lt $bitmap.Height; $y++) {
            for ($x = 0; $x -lt $bitmap.Width; $x++) {
                $pixel = $bitmap.GetPixel($x, $y)
                $pixels[$offset++] = $pixel.A
                $pixels[$offset++] = $pixel.R
                $pixels[$offset++] = $pixel.G
                $pixels[$offset++] = $pixel.B
            }
        }
        return [Convert]::ToHexString(
            [Security.Cryptography.SHA256]::HashData($pixels)
        )
    }
    finally {
        $bitmap.Dispose()
        $stream.Dispose()
    }
}

function Get-ImageFilePixelHash {
    param([Parameter(Mandatory)][string]$Path)

    return Get-ImagePixelHash -Bytes ([IO.File]::ReadAllBytes($Path))
}

function Get-IconFramePixelHashes {
    param([Parameter(Mandatory)][string]$Path)

    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 6) {
        throw 'Cannot inspect pixel content of a truncated ICO.'
    }
    $count = [int][BitConverter]::ToUInt16($bytes, 4)
    $directoryEnd = 6 + (16 * $count)
    if ($bytes.Length -lt $directoryEnd) {
        throw 'Cannot inspect pixel content of an ICO with a truncated directory.'
    }
    $hashes = @()
    for ($index = 0; $index -lt $count; $index++) {
        $entryOffset = 6 + (16 * $index)
        $length = [uint64][BitConverter]::ToUInt32($bytes, $entryOffset + 8)
        $imageOffset = [uint64][BitConverter]::ToUInt32($bytes, $entryOffset + 12)
        if ($length -gt [uint64][int]::MaxValue -or
            $imageOffset -gt [uint64][int]::MaxValue -or
            ($imageOffset + $length) -gt [uint64]$bytes.Length) {
            throw 'Cannot inspect pixel content of an ICO frame outside its payload.'
        }
        $payload = [byte[]]::new([int]$length)
        [Array]::Copy($bytes, [int]$imageOffset, $payload, 0, [int]$length)
        $hashes += Get-ImagePixelHash -Bytes $payload
    }
    return $hashes
}

function Read-BigEndianUInt32 {
    param(
        [Parameter(Mandatory)][byte[]]$Bytes,
        [Parameter(Mandatory)][int]$Offset
    )

    return [uint32]((([uint32]$Bytes[$Offset]) -shl 24) -bor
        (([uint32]$Bytes[$Offset + 1]) -shl 16) -bor
        (([uint32]$Bytes[$Offset + 2]) -shl 8) -bor
        [uint32]$Bytes[$Offset + 3])
}

function Get-ReviewedIconEvidence {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][int[]]$ExpectedFrames,
        [Parameter(Mandatory)][string]$ExpectedHash
    )

    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 6 -or [BitConverter]::ToUInt16($bytes, 0) -ne 0 -or
        [BitConverter]::ToUInt16($bytes, 2) -ne 1) {
        throw 'ZiFile.ico has an invalid ICONDIR header.'
    }
    $count = [int][BitConverter]::ToUInt16($bytes, 4)
    if ($count -ne $ExpectedFrames.Count) {
        throw "ZiFile.ico must contain exactly $($ExpectedFrames.Count) frames, found $count."
    }
    $directoryEnd = 6 + (16 * $count)
    if ($bytes.Length -lt $directoryEnd) {
        throw 'ZiFile.ico has a truncated frame directory.'
    }

    $frames = @()
    for ($index = 0; $index -lt $count; $index++) {
        $entryOffset = 6 + (16 * $index)
        $width = if ($bytes[$entryOffset] -eq 0) { 256 } else { [int]$bytes[$entryOffset] }
        $height = if ($bytes[$entryOffset + 1] -eq 0) { 256 } else { [int]$bytes[$entryOffset + 1] }
        $planes = [BitConverter]::ToUInt16($bytes, $entryOffset + 4)
        $bitsPerPixel = [BitConverter]::ToUInt16($bytes, $entryOffset + 6)
        $length = [uint64][BitConverter]::ToUInt32($bytes, $entryOffset + 8)
        $imageOffset = [uint64][BitConverter]::ToUInt32($bytes, $entryOffset + 12)
        $expectedSize = $ExpectedFrames[$index]
        if ($width -ne $expectedSize -or $height -ne $expectedSize) {
            throw "ZiFile.ico frame $index must be ${expectedSize}x${expectedSize}, found ${width}x${height}."
        }
        if ($planes -ne 1 -or $bitsPerPixel -ne 32) {
            throw "ZiFile.ico frame $expectedSize must use one 32-bit color plane."
        }
        if ($imageOffset -lt $directoryEnd -or $length -lt 24 -or
            ($imageOffset + $length) -gt [uint64]$bytes.Length) {
            throw "ZiFile.ico frame $expectedSize has invalid payload bounds."
        }
        $pngSignature = [byte[]](0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)
        for ($signatureIndex = 0; $signatureIndex -lt $pngSignature.Count; $signatureIndex++) {
            if ($bytes[[int]$imageOffset + $signatureIndex] -ne $pngSignature[$signatureIndex]) {
                throw "ZiFile.ico frame $expectedSize is not PNG encoded."
            }
        }
        $chunkType = [Text.Encoding]::ASCII.GetString($bytes, [int]$imageOffset + 12, 4)
        $ihdrLength = Read-BigEndianUInt32 -Bytes $bytes -Offset ([int]$imageOffset + 8)
        $pngWidth = Read-BigEndianUInt32 -Bytes $bytes -Offset ([int]$imageOffset + 16)
        $pngHeight = Read-BigEndianUInt32 -Bytes $bytes -Offset ([int]$imageOffset + 20)
        if ($chunkType -cne 'IHDR' -or $ihdrLength -ne 13 -or
            $pngWidth -ne $expectedSize -or $pngHeight -ne $expectedSize) {
            throw "ZiFile.ico frame $expectedSize has invalid PNG geometry."
        }
        $frames += [pscustomobject]@{
            size = $expectedSize
            bits_per_pixel = $bitsPerPixel
            encoding = 'png'
            payload_bytes = $length
        }
    }

    $actualHash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    if ($actualHash -cne $ExpectedHash) {
        throw 'ZiFile.ico hash does not match its reviewed pinned value.'
    }
    return [pscustomobject]@{
        name = [IO.Path]::GetFileName($Path)
        sha256 = $actualHash
        frames = $frames
    }
}

$expectedGeometry = [ordered]@{
    'Square44x44Logo.png' = 44
    'Square50x50Logo.png' = 50
    'StoreLogo.png' = 50
    'Square150x150Logo.png' = 150
    'Square310x310Logo.png' = 310
    'Square44x44Logo.scale-100.png' = 44
    'Square44x44Logo.scale-200.png' = 88
    'Square44x44Logo.scale-400.png' = 176
    'Square150x150Logo.scale-100.png' = 150
    'Square150x150Logo.scale-200.png' = 300
    'Square150x150Logo.scale-400.png' = 600
    'StoreLogo.scale-100.png' = 50
    'StoreLogo.scale-125.png' = 63
    'StoreLogo.scale-150.png' = 75
    'StoreLogo.scale-200.png' = 100
    'StoreLogo.scale-400.png' = 200
}
$targetSizes = @(16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256)
$targetForms = @('', '_altform-unplated', '_altform-lightunplated')
foreach ($size in $targetSizes) {
    foreach ($form in $targetForms) {
        $expectedGeometry["Square44x44Logo.targetsize-$size$form.png"] = $size
    }
}

$catalogData = Get-Content -Raw -LiteralPath $catalog | ConvertFrom-Json
if ($catalogData.schema_version -ne 2 -or
    $catalogData.specification -cne 'https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-construction') {
    throw 'MSIX asset catalog has an unsupported schema or specification source.'
}
$expectedIconFrames = @(16, 24, 32, 48, 256)
if ([string]$catalogData.icon.name -cne 'ZiFile.ico' -or
    [string]$catalogData.icon.sha256 -notmatch '^[A-F0-9]{64}$' -or
    @($catalogData.icon.frames).Count -ne $expectedIconFrames.Count -or
    (Compare-Object -ReferenceObject $expectedIconFrames -DifferenceObject @($catalogData.icon.frames))) {
    throw 'MSIX asset catalog has an invalid reviewed Win32 icon definition.'
}
$catalogEntries = @($catalogData.assets)
if ($catalogEntries.Count -ne $expectedGeometry.Count) {
    throw "MSIX asset catalog must contain exactly $($expectedGeometry.Count) reviewed PNG entries."
}
$catalogByName = @{}
foreach ($entry in $catalogEntries) {
    if ([string]::IsNullOrWhiteSpace($entry.name) -or $catalogByName.ContainsKey([string]$entry.name)) {
        throw "MSIX asset catalog contains an empty or duplicate name: $($entry.name)"
    }
    $catalogByName[[string]$entry.name] = $entry
}

$expectedPngs = [ordered]@{}
foreach ($geometry in $expectedGeometry.GetEnumerator()) {
    if (-not $catalogByName.ContainsKey($geometry.Key)) {
        throw "MSIX asset catalog omits required qualified asset: $($geometry.Key)"
    }
    $entry = $catalogByName[$geometry.Key]
    if ($entry.width -ne $geometry.Value -or $entry.height -ne $geometry.Value -or
        [string]$entry.sha256 -notmatch '^[A-F0-9]{64}$') {
        throw "MSIX asset catalog has invalid geometry or SHA-256 for $($geometry.Key)."
    }
    $expectedPngs[$geometry.Key] = @{ Size = $geometry.Value; Sha256 = [string]$entry.sha256 }
}
$unexpectedCatalogEntries = @($catalogEntries | Where-Object { -not $expectedGeometry.Contains([string]$_.name) })
if ($unexpectedCatalogEntries.Count -gt 0) {
    throw "MSIX asset catalog contains unexpected entries: $($unexpectedCatalogEntries.name -join ', ')"
}
$actualPngNames = @(Get-ChildItem -LiteralPath $assets -File -Filter '*.png' | ForEach-Object Name)
$unexpectedPngs = @($actualPngNames | Where-Object { -not $expectedGeometry.Contains($_) })
if ($actualPngNames.Count -ne $expectedGeometry.Count -or $unexpectedPngs.Count -gt 0) {
    throw "MSIX asset directory contains an incomplete or unexpected PNG set: $($unexpectedPngs -join ', ')"
}
$assetEvidence = @()
foreach ($entry in $expectedPngs.GetEnumerator()) {
    $path = Join-Path $assets $entry.Key
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required MSIX PNG asset is missing: $($entry.Key)"
    }
    $bitmap = [Drawing.Bitmap]::new($path)
    try {
        if ($bitmap.RawFormat.Guid -ne [Drawing.Imaging.ImageFormat]::Png.Guid) {
            throw "MSIX asset is not a PNG image: $($entry.Key)"
        }
        if ($bitmap.Width -ne $entry.Value.Size -or $bitmap.Height -ne $entry.Value.Size) {
            throw "MSIX asset $($entry.Key) must be $($entry.Value.Size)x$($entry.Value.Size), found $($bitmap.Width)x$($bitmap.Height)."
        }
        if (-not [Drawing.Image]::IsAlphaPixelFormat($bitmap.PixelFormat)) {
            throw "MSIX asset must retain an alpha-capable pixel format: $($entry.Key)"
        }
        $hasTransparentPixel = $false
        $hasOpaquePixel = $false
        for ($y = 0; $y -lt $bitmap.Height -and -not ($hasTransparentPixel -and $hasOpaquePixel); $y++) {
            for ($x = 0; $x -lt $bitmap.Width -and -not ($hasTransparentPixel -and $hasOpaquePixel); $x++) {
                $alpha = $bitmap.GetPixel($x, $y).A
                if ($alpha -eq 0) { $hasTransparentPixel = $true }
                if ($alpha -eq 255) { $hasOpaquePixel = $true }
            }
        }
        if (-not $hasTransparentPixel -or -not $hasOpaquePixel) {
            throw "MSIX asset must contain both transparent and opaque pixels: $($entry.Key)"
        }
        $actualHash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        if ($actualHash -cne $entry.Value.Sha256) {
            throw "MSIX asset hash does not match its reviewed pinned value: $($entry.Key)"
        }
        $assetEvidence += [pscustomobject]@{
            name = $entry.Key
            width = $bitmap.Width
            height = $bitmap.Height
            sha256 = $actualHash
        }
    }
    finally {
        $bitmap.Dispose()
    }
}

$iconPath = Join-Path $assets 'ZiFile.ico'
if (-not (Test-Path -LiteralPath $iconPath -PathType Leaf)) {
    throw 'Required desktop icon is missing: ZiFile.ico'
}
$iconEvidence = Get-ReviewedIconEvidence `
    -Path $iconPath `
    -ExpectedFrames $expectedIconFrames `
    -ExpectedHash ([string]$catalogData.icon.sha256)

[xml]$manifestXml = Get-Content -Raw -LiteralPath $manifest
$namespace = [Xml.XmlNamespaceManager]::new($manifestXml.NameTable)
$namespace.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
$namespace.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
$propertiesLogo = $manifestXml.SelectSingleNode('/f:Package/f:Properties/f:Logo', $namespace)
$visualElements = $manifestXml.SelectSingleNode('/f:Package/f:Applications/f:Application/uap:VisualElements', $namespace)
$fileAssociationLogo = $manifestXml.SelectSingleNode(
    '/f:Package/f:Applications/f:Application/f:Extensions/uap:Extension/uap:FileTypeAssociation/uap:Logo',
    $namespace
)
if ($null -eq $propertiesLogo -or $null -eq $visualElements -or $null -eq $fileAssociationLogo) {
    throw 'AppxManifest.xml is missing a required ZiFile logo declaration.'
}
$references = @(
    [string]$propertiesLogo.InnerText,
    [string]$visualElements.GetAttribute('Square44x44Logo'),
    [string]$visualElements.GetAttribute('Square150x150Logo'),
    [string]$fileAssociationLogo.InnerText
)
foreach ($reference in $references) {
    if ([string]::IsNullOrWhiteSpace($reference)) {
        throw 'AppxManifest.xml contains an empty required logo reference.'
    }
    $resolvedReference = Join-Path (Split-Path -Parent $assets) $reference
    if (-not (Test-Path -LiteralPath $resolvedReference -PathType Leaf)) {
        throw "AppxManifest.xml logo reference does not exist: $reference"
    }
}

$generatorMatches = $null
if ($VerifyGenerator) {
    $temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $fixture = [IO.Path]::GetFullPath((Join-Path $temporaryBase "zifile-msix-assets-$([Guid]::NewGuid().ToString('N'))"))
    if (-not $fixture.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to create the asset fixture outside the system temporary directory.'
    }
    try {
        & $generator -OutputDirectory $fixture -SkipStoreListingAsset | Out-Null
        foreach ($name in @($expectedPngs.Keys) + @('ZiFile.ico')) {
            $committedHash = (Get-FileHash -LiteralPath (Join-Path $assets $name) -Algorithm SHA256).Hash
            $generatedHash = (Get-FileHash -LiteralPath (Join-Path $fixture $name) -Algorithm SHA256).Hash
            if ($committedHash -cne $generatedHash) {
                if ($name -eq 'ZiFile.ico') {
                    $committedPixels = @(Get-IconFramePixelHashes -Path (Join-Path $assets $name))
                    $generatedPixels = @(Get-IconFramePixelHashes -Path (Join-Path $fixture $name))
                }
                else {
                    $committedPixels = @(Get-ImageFilePixelHash -Path (Join-Path $assets $name))
                    $generatedPixels = @(Get-ImageFilePixelHash -Path (Join-Path $fixture $name))
                }
                if (($committedPixels -join ',') -cne ($generatedPixels -join ',')) {
                    throw "Committed MSIX asset differs from deterministic generator output: $name"
                }
            }
        }
        $generatorMatches = $true
    }
    finally {
        if (Test-Path -LiteralPath $fixture) {
            $resolvedFixture = [IO.Path]::GetFullPath($fixture)
            if (-not $resolvedFixture.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase) -or
                [IO.Path]::GetFileName($resolvedFixture) -notlike 'zifile-msix-assets-*') {
                throw "Refusing to remove unexpected asset fixture: $resolvedFixture"
            }
            Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
        }
    }
}

[pscustomobject]@{
    schema_version = 2
    validated = $true
    png_count = $assetEvidence.Count
    scale_asset_count = @($assetEvidence | Where-Object name -Match '\.scale-').Count
    app_list_target_asset_count = @($assetEvidence | Where-Object name -Match '\.targetsize-').Count
    app_list_target_sizes = $targetSizes
    app_list_theme_variants = $targetForms.Count
    icon_frames = $expectedIconFrames
    icon_sha256 = $iconEvidence.sha256
    icon = $iconEvidence
    manifest_logo_references = $references.Count
    hashes_pinned = $true
    generator_matches_on_current_host = $generatorMatches
    assets = $assetEvidence
} | ConvertTo-Json -Depth 4
