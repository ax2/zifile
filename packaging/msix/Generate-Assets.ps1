param(
    [string]$OutputDirectory = (Join-Path $PSScriptRoot 'Assets'),
    [string]$StoreOutputDirectory = (Join-Path $PSScriptRoot '..\store\listing-assets'),
    [switch]$SkipStoreListingAsset
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null

function New-ZiFileBitmap {
    param([int]$Size)

    $bitmap = [System.Drawing.Bitmap]::new($Size, $Size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.Clear([System.Drawing.Color]::Transparent)

    $scale = $Size / 256.0
    $background = [System.Drawing.RectangleF]::new(12 * $scale, 12 * $scale, 232 * $scale, 232 * $scale)
    $radius = 54 * $scale
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $diameter = 2 * $radius
    $path.AddArc($background.Left, $background.Top, $diameter, $diameter, 180, 90)
    $path.AddArc($background.Right - $diameter, $background.Top, $diameter, $diameter, 270, 90)
    $path.AddArc($background.Right - $diameter, $background.Bottom - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc($background.Left, $background.Bottom - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()
    $gradient = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $background,
        [System.Drawing.Color]::FromArgb(54, 209, 220),
        [System.Drawing.Color]::FromArgb(36, 116, 232),
        45
    )
    $graphics.FillPath($gradient, $path)

    $folder = [System.Drawing.RectangleF]::new(58 * $scale, 82 * $scale, 140 * $scale, 121 * $scale)
    $folderBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(245, 250, 255))
    $folderRadius = [System.Drawing.SizeF]::new(18 * $scale, 18 * $scale)
    $graphics.FillRoundedRectangle($folderBrush, $folder, $folderRadius)
    $tabBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(245, 250, 255))
    $graphics.FillRectangle($tabBrush, 77 * $scale, 68 * $scale, 58 * $scale, 31 * $scale)

    $zipperPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(24, 92, 199), 12 * $scale)
    $graphics.DrawLine($zipperPen, 128 * $scale, 72 * $scale, 128 * $scale, 188 * $scale)
    $toothBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(54, 187, 211))
    for ($index = 0; $index -lt 5; $index++) {
        $x = if ($index % 2 -eq 0) { 128 } else { 112 }
        $graphics.FillRectangle($toothBrush, $x * $scale, (86 + 24 * $index) * $scale, 16 * $scale, 16 * $scale)
    }
    $pullBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(24, 92, 199))
    $graphics.FillRoundedRectangle(
        $pullBrush,
        [System.Drawing.RectangleF]::new(111 * $scale, 171 * $scale, 34 * $scale, 22 * $scale),
        [System.Drawing.SizeF]::new(8 * $scale, 8 * $scale)
    )

    foreach ($resource in @($pullBrush, $toothBrush, $zipperPen, $tabBrush, $folderBrush, $gradient, $path, $graphics)) {
        $resource.Dispose()
    }
    return $bitmap
}

$assets = @{
    'Square44x44Logo.png' = 44
    'Square50x50Logo.png' = 50
    'Square150x150Logo.png' = 150
    'Square310x310Logo.png' = 310
    'StoreLogo.png' = 50
}

foreach ($asset in $assets.GetEnumerator()) {
    $bitmap = New-ZiFileBitmap -Size $asset.Value
    $bitmap.Save((Join-Path $OutputDirectory $asset.Key), [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

if (-not $SkipStoreListingAsset) {
    New-Item -ItemType Directory -Path $StoreOutputDirectory -Force | Out-Null
    $storeBitmap = New-ZiFileBitmap -Size 300
    $storeBitmap.Save(
        (Join-Path $StoreOutputDirectory 'AppTile300x300.png'),
        [System.Drawing.Imaging.ImageFormat]::Png
    )
    $storeBitmap.Dispose()
}

$iconBitmap = New-ZiFileBitmap -Size 256
$iconHandle = $iconBitmap.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($iconHandle)
$stream = [System.IO.File]::Create((Join-Path $OutputDirectory 'ZiFile.ico'))
$icon.Save($stream)
$stream.Dispose()
$icon.Dispose()
$iconBitmap.Dispose()

Write-Host "Generated ZiFile package assets in $OutputDirectory"
if (-not $SkipStoreListingAsset) {
    Write-Host "Generated ZiFile Store listing asset in $StoreOutputDirectory"
}
