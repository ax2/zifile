[CmdletBinding()]
param(
    [string]$AssetsDirectory = (Join-Path $PSScriptRoot 'Assets'),
    [string]$ManifestPath = (Join-Path $PSScriptRoot 'AppxManifest.xml'),
    [string]$GeneratorPath = (Join-Path $PSScriptRoot 'Generate-Assets.ps1'),
    [switch]$SkipReproducibility
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$assets = [IO.Path]::GetFullPath($AssetsDirectory)
$manifest = [IO.Path]::GetFullPath($ManifestPath)
$generator = [IO.Path]::GetFullPath($GeneratorPath)
if (-not (Test-Path -LiteralPath $assets -PathType Container)) {
    throw "MSIX asset directory does not exist: $assets"
}
foreach ($file in @($manifest, $generator)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required MSIX asset input does not exist: $file"
    }
}

Add-Type -AssemblyName System.Drawing
$expectedPngs = [ordered]@{
    'Square44x44Logo.png' = 44
    'Square50x50Logo.png' = 50
    'StoreLogo.png' = 50
    'Square150x150Logo.png' = 150
    'Square310x310Logo.png' = 310
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
        if ($bitmap.Width -ne $entry.Value -or $bitmap.Height -ne $entry.Value) {
            throw "MSIX asset $($entry.Key) must be $($entry.Value)x$($entry.Value), found $($bitmap.Width)x$($bitmap.Height)."
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
        $assetEvidence += [pscustomobject]@{
            name = $entry.Key
            width = $bitmap.Width
            height = $bitmap.Height
            sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
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
$icon = [Drawing.Icon]::new($iconPath)
try {
    if ($icon.Width -ne 256 -or $icon.Height -ne 256) {
        throw "ZiFile.ico must expose a 256x256 icon, found $($icon.Width)x$($icon.Height)."
    }
}
finally {
    $icon.Dispose()
}

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

$reproducible = $null
if (-not $SkipReproducibility) {
    $temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $fixture = [IO.Path]::GetFullPath((Join-Path $temporaryBase "zifile-msix-assets-$([Guid]::NewGuid().ToString('N'))"))
    if (-not $fixture.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to create the asset fixture outside the system temporary directory.'
    }
    try {
        & $generator -OutputDirectory $fixture | Out-Null
        foreach ($name in @($expectedPngs.Keys) + @('ZiFile.ico')) {
            $committedHash = (Get-FileHash -LiteralPath (Join-Path $assets $name) -Algorithm SHA256).Hash
            $generatedHash = (Get-FileHash -LiteralPath (Join-Path $fixture $name) -Algorithm SHA256).Hash
            if ($committedHash -cne $generatedHash) {
                throw "Committed MSIX asset differs from deterministic generator output: $name"
            }
        }
        $reproducible = $true
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
    schema_version = 1
    validated = $true
    png_count = $assetEvidence.Count
    icon_size = '256x256'
    manifest_logo_references = $references.Count
    reproducible = $reproducible
    assets = $assetEvidence
} | ConvertTo-Json -Depth 4
