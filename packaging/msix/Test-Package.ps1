param(
    [Parameter(Mandatory)][string]$PackagePath,
    [ValidateSet('x64', 'arm64')]
    [Parameter(Mandatory)][string]$Architecture,
    [ValidatePattern('^\d+\.\d+\.\d+\.\d+$')]
    [Parameter(Mandatory)][string]$ExpectedVersion,
    [Parameter(Mandatory)][string]$ExpectedIdentityName,
    [Parameter(Mandatory)][string]$ExpectedPublisher,
    [Parameter(Mandatory)][string]$ExpectedPublisherDisplayName,
    [Parameter(Mandatory)][string]$ExpectedMinimumVersion,
    [string]$EvidencePath,
    [switch]$RequireSignature
)

$ErrorActionPreference = 'Stop'
$package = [System.IO.Path]::GetFullPath($PackagePath)
if (-not (Test-Path -LiteralPath $package -PathType Leaf)) {
    throw "MSIX package does not exist: $package"
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

$temporaryRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$auditRoot = [System.IO.Path]::GetFullPath((Join-Path $temporaryRoot (
    'zifile-msix-audit-{0}' -f [Guid]::NewGuid().ToString('N')
)))
if (-not $auditRoot.StartsWith($temporaryRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to stage the MSIX audit outside the system temporary directory.'
}

function Get-PeMachine {
    param([Parameter(Mandatory)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $reader = [System.IO.BinaryReader]::new($stream)
        try {
            if ($reader.ReadUInt16() -ne 0x5A4D) {
                throw "Not a PE executable: $Path"
            }
            $stream.Position = 0x3C
            $peOffset = $reader.ReadUInt32()
            if ($peOffset -gt ($stream.Length - 6)) {
                throw "Invalid PE header offset: $Path"
            }
            $stream.Position = $peOffset
            if ($reader.ReadUInt32() -ne 0x00004550) {
                throw "Invalid PE signature: $Path"
            }
            return $reader.ReadUInt16()
        }
        finally {
            $reader.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

try {
    New-Item -ItemType Directory -Path $auditRoot -Force | Out-Null
    & $makeAppx.FullName unpack /p $package /d $auditRoot /o | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw 'MakeAppx failed to unpack the MSIX package.'
    }

    $manifestPath = Join-Path $auditRoot 'AppxManifest.xml'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw 'MSIX package does not contain AppxManifest.xml.'
    }
    [xml]$manifest = Get-Content -Raw -LiteralPath $manifestPath
    $namespace = [System.Xml.XmlNamespaceManager]::new($manifest.NameTable)
    $namespace.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
    $namespace.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
    $namespace.AddNamespace('uap3', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/3')
    $namespace.AddNamespace('desktop', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10')
    $namespace.AddNamespace('com', 'http://schemas.microsoft.com/appx/manifest/com/windows10')
    $namespace.AddNamespace('desktop4', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10/4')
    $namespace.AddNamespace('desktop5', 'http://schemas.microsoft.com/appx/manifest/desktop/windows10/5')

    $identity = $manifest.SelectSingleNode('/f:Package/f:Identity', $namespace)
    if (-not $identity) { throw 'MSIX manifest has no package identity.' }
    if ($identity.Name -cne $ExpectedIdentityName) {
        throw "Identity mismatch: expected '$ExpectedIdentityName', found '$($identity.Name)'."
    }
    if ($identity.Publisher -cne $ExpectedPublisher) {
        throw "Publisher mismatch: expected '$ExpectedPublisher', found '$($identity.Publisher)'."
    }
    $publisherDisplayName = $manifest.SelectSingleNode(
        '/f:Package/f:Properties/f:PublisherDisplayName',
        $namespace
    )
    if (-not $publisherDisplayName) { throw 'MSIX manifest has no PublisherDisplayName.' }
    if ($publisherDisplayName.InnerText -cne $ExpectedPublisherDisplayName) {
        throw "PublisherDisplayName mismatch: expected '$ExpectedPublisherDisplayName', found '$($publisherDisplayName.InnerText)'."
    }
    if ($identity.Version -cne $ExpectedVersion) {
        throw "Version mismatch: expected '$ExpectedVersion', found '$($identity.Version)'."
    }
    if ($identity.ProcessorArchitecture -cne $Architecture) {
        throw "Architecture mismatch: expected '$Architecture', found '$($identity.ProcessorArchitecture)'."
    }
    $targetFamily = $manifest.SelectSingleNode(
        "/f:Package/f:Dependencies/f:TargetDeviceFamily[@Name='Windows.Desktop']",
        $namespace
    )
    if (-not $targetFamily) { throw 'MSIX manifest has no Windows.Desktop target family.' }
    if ($targetFamily.MinVersion -cne $ExpectedMinimumVersion) {
        throw "Minimum Windows version mismatch: expected '$ExpectedMinimumVersion', found '$($targetFamily.MinVersion)'."
    }

    $requiredExecutables = @(
        'ZiFile\zifile-desktop.exe',
        'ZiFile\zifile.exe',
        'ZiFile\zifile-worker.exe',
        'ZiFile\zifile-shell.dll'
    )
    $expectedMachine = if ($Architecture -eq 'x64') { 0x8664 } else { 0xAA64 }
    $machineEvidence = [ordered]@{}
    foreach ($relativePath in $requiredExecutables) {
        $executable = Join-Path $auditRoot $relativePath
        if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
            throw "MSIX package is missing required executable: $relativePath"
        }
        $machine = Get-PeMachine -Path $executable
        if ($machine -ne $expectedMachine) {
            throw ('PE machine mismatch for {0}: expected 0x{1:X4}, found 0x{2:X4}.' -f
                $relativePath, $expectedMachine, $machine)
        }
        $machineEvidence[$relativePath] = ('0x{0:X4}' -f $machine)
    }

    $alias = $manifest.SelectSingleNode(
        "//uap3:AppExecutionAlias/desktop:ExecutionAlias[@Alias='zifile.exe']",
        $namespace
    )
    if (-not $alias) { throw 'MSIX manifest does not register the zifile.exe app execution alias.' }

    $shellClsid = '2F86F25D-3B76-4CD2-8FE8-9D7A2EEFB531'
    $shellClass = $manifest.SelectSingleNode(
        "//com:SurrogateServer/com:Class[@Id='$shellClsid']",
        $namespace
    )
    if (-not $shellClass -or $shellClass.Path -cne 'ZiFile\zifile-shell.dll' -or $shellClass.ThreadingModel -cne 'STA') {
        throw 'MSIX manifest does not register the ZiFile STA shell COM class and DLL path.'
    }
    $shellItemTypes = @(
        $manifest.SelectNodes(
            "//desktop4:FileExplorerContextMenus/desktop5:ItemType[desktop5:Verb[@Clsid='$shellClsid']]",
            $namespace
        ) | ForEach-Object { $_.Type }
    )
    foreach ($itemType in @('*', 'Directory')) {
        if ($shellItemTypes -cnotcontains $itemType) {
            throw "MSIX manifest is missing ZiFile shell command item type: $itemType"
        }
    }
    $extractShellClsid = '2D39AD2E-1B36-4F4F-8E09-589F0B1D2BC3'
    $extractShellClass = $manifest.SelectSingleNode(
        "//com:SurrogateServer/com:Class[@Id='$extractShellClsid']",
        $namespace
    )
    if (-not $extractShellClass -or $extractShellClass.Path -cne 'ZiFile\zifile-shell.dll' -or
        $extractShellClass.ThreadingModel -cne 'STA') {
        throw 'MSIX manifest does not register the ZiFile extract STA shell COM class.'
    }
    $extractShellItemTypes = @(
        $manifest.SelectNodes(
            "//desktop4:FileExplorerContextMenus/desktop5:ItemType[desktop5:Verb[@Clsid='$extractShellClsid']]",
            $namespace
        ) | ForEach-Object { $_.Type }
    )
    if ($extractShellItemTypes -cnotcontains '*') {
        throw 'MSIX manifest does not register the ZiFile extract command for file selections.'
    }

    $requiredExtensions = @(
        '.zip', '.zipx', '.7z', '.cbz', '.cb7', '.rar', '.cbr', '.cab', '.tar', '.cbt',
        '.gz', '.tgz', '.zst', '.tzst', '.xz', '.txz', '.lzma', '.bz', '.bz2', '.tbz',
        '.tbz2', '.lz4', '.br'
    )
    $declaredExtensions = @(
        $manifest.SelectNodes('//uap:FileTypeAssociation/uap:SupportedFileTypes/uap:FileType', $namespace) |
            ForEach-Object { $_.InnerText }
    )
    foreach ($extension in $requiredExtensions) {
        if ($declaredExtensions -cnotcontains $extension) {
            throw "MSIX manifest is missing file association: $extension"
        }
    }

    $assetCatalogPath = Join-Path $PSScriptRoot 'assets.json'
    if (-not (Test-Path -LiteralPath $assetCatalogPath -PathType Leaf)) {
        throw 'The reviewed MSIX asset catalog is unavailable during package audit.'
    }
    $assetCatalog = Get-Content -Raw -LiteralPath $assetCatalogPath | ConvertFrom-Json
    if ($assetCatalog.schema_version -ne 2) {
        throw 'The reviewed MSIX asset catalog schema is unsupported during package audit.'
    }
    $packagedAssetEvidence = @()
    foreach ($asset in @($assetCatalog.assets)) {
        $packagedAssetPath = Join-Path $auditRoot (Join-Path 'Assets' ([string]$asset.name))
        if (-not (Test-Path -LiteralPath $packagedAssetPath -PathType Leaf)) {
            throw "MSIX package is missing reviewed visual asset: $($asset.name)"
        }
        $packagedAssetHash = (Get-FileHash -LiteralPath $packagedAssetPath -Algorithm SHA256).Hash
        if ($packagedAssetHash -cne [string]$asset.sha256) {
            throw "MSIX package visual asset differs from its reviewed hash: $($asset.name)"
        }
        $packagedAssetEvidence += [pscustomobject]@{
            name = [string]$asset.name
            sha256 = $packagedAssetHash
        }
    }
    if ($packagedAssetEvidence.Count -ne 58) {
        throw "MSIX package must contain 58 reviewed PNG assets, found $($packagedAssetEvidence.Count)."
    }
    $reviewedIcon = $assetCatalog.icon
    $packagedIconPath = Join-Path $auditRoot (Join-Path 'Assets' ([string]$reviewedIcon.name))
    if (-not (Test-Path -LiteralPath $packagedIconPath -PathType Leaf)) {
        throw "MSIX package is missing reviewed desktop icon: $($reviewedIcon.name)"
    }
    $packagedIconHash = (Get-FileHash -LiteralPath $packagedIconPath -Algorithm SHA256).Hash
    if ($packagedIconHash -cne [string]$reviewedIcon.sha256) {
        throw "MSIX package desktop icon differs from its reviewed hash: $($reviewedIcon.name)"
    }
    $packagedIconEvidence = [pscustomobject]@{
        name = [string]$reviewedIcon.name
        frames = @($reviewedIcon.frames)
        sha256 = $packagedIconHash
    }

    $forbiddenFiles = @(Get-ChildItem -LiteralPath $auditRoot -Recurse -File | Where-Object {
        $_.Extension -in @('.pfx', '.p12', '.pem', '.key', '.zip')
    })
    if ($forbiddenFiles.Count -gt 0) {
        throw "MSIX contains forbidden files: $($forbiddenFiles.Name -join ', ')"
    }

    $signature = Get-AuthenticodeSignature -LiteralPath $package
    if ($RequireSignature -and $signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "MSIX signature is required but status is '$($signature.Status)'."
    }

    $evidence = [pscustomobject]@{
        schema_version = 2
        package = [System.IO.Path]::GetFileName($package)
        sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash
        identity = $identity.Name
        publisher = $identity.Publisher
        publisher_display_name = $publisherDisplayName.InnerText
        version = $identity.Version
        architecture = $identity.ProcessorArchitecture
        minimum_windows_version = $targetFamily.MinVersion
        pe_machines = $machineEvidence
        file_associations = $declaredExtensions
        reviewed_visual_assets = $packagedAssetEvidence
        reviewed_desktop_icon = $packagedIconEvidence
        app_execution_alias = 'zifile.exe'
        shell_extension = [pscustomobject]@{
            clsid = $shellClsid
            path = $shellClass.Path
            threading_model = $shellClass.ThreadingModel
            item_types = $shellItemTypes
            extract_clsid = $extractShellClsid
            extract_path = $extractShellClass.Path
            extract_threading_model = $extractShellClass.ThreadingModel
            extract_item_types = $extractShellItemTypes
        }
        forbidden_file_count = $forbiddenFiles.Count
        signature_required = [bool]$RequireSignature
        signature_status = $signature.Status.ToString()
    }
    $evidenceJson = $evidence | ConvertTo-Json -Depth 5
    if ($EvidencePath) {
        $resolvedEvidencePath = [System.IO.Path]::GetFullPath($EvidencePath)
        $evidenceParent = Split-Path -Parent $resolvedEvidencePath
        if (-not (Test-Path -LiteralPath $evidenceParent -PathType Container)) {
            throw "Evidence output directory does not exist: $evidenceParent"
        }
        Set-Content -LiteralPath $resolvedEvidencePath -Value $evidenceJson -Encoding utf8
    }
    $evidenceJson
}
finally {
    if (
        (Test-Path -LiteralPath $auditRoot) -and
        $auditRoot.StartsWith($temporaryRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
        ([System.IO.Path]::GetFileName($auditRoot) -like 'zifile-msix-audit-*')
    ) {
        Remove-Item -LiteralPath $auditRoot -Recurse -Force
    }
}
