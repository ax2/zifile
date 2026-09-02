[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ExecutablePath,
    [int[]]$ExpectedFrames = @(16, 24, 32, 48, 256)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$executable = [IO.Path]::GetFullPath($ExecutablePath)
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Executable does not exist for embedded icon audit: $executable"
}
if ($ExpectedFrames.Count -eq 0 -or @($ExpectedFrames | Where-Object { $_ -lt 1 -or $_ -gt 256 }).Count -gt 0) {
    throw 'Embedded icon audit requires frame sizes between 1 and 256.'
}

if (-not ('ZiFile.PackageAudit.NativeResources' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

namespace ZiFile.PackageAudit
{
    public static class NativeResources
    {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        public static extern IntPtr LoadLibraryEx(string fileName, IntPtr file, uint flags);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool FreeLibrary(IntPtr module);

        [DllImport("kernel32.dll", EntryPoint = "FindResourceW", SetLastError = true)]
        public static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);

        [DllImport("kernel32.dll", SetLastError = true)]
        public static extern uint SizeofResource(IntPtr module, IntPtr resource);

        [DllImport("kernel32.dll", SetLastError = true)]
        public static extern IntPtr LoadResource(IntPtr module, IntPtr resource);

        [DllImport("kernel32.dll")]
        public static extern IntPtr LockResource(IntPtr resourceData);
    }
}
'@
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

function Get-ResourceBytes {
    param(
        [Parameter(Mandatory)][IntPtr]$Module,
        [Parameter(Mandatory)][int]$Name,
        [Parameter(Mandatory)][int]$Type,
        [Parameter(Mandatory)][string]$Description
    )

    $resource = [ZiFile.PackageAudit.NativeResources]::FindResource(
        $Module,
        [IntPtr]::new($Name),
        [IntPtr]::new($Type)
    )
    if ($resource -eq [IntPtr]::Zero) {
        $errorCode = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "$Description was not found (Win32 error $errorCode)."
    }
    $size = [ZiFile.PackageAudit.NativeResources]::SizeofResource($Module, $resource)
    if ($size -eq 0 -or $size -gt [int]::MaxValue) {
        throw "$Description has an invalid resource size: $size."
    }
    $resourceData = [ZiFile.PackageAudit.NativeResources]::LoadResource($Module, $resource)
    $pointer = if ($resourceData -eq [IntPtr]::Zero) {
        [IntPtr]::Zero
    } else {
        [ZiFile.PackageAudit.NativeResources]::LockResource($resourceData)
    }
    if ($pointer -eq [IntPtr]::Zero) {
        throw "$Description could not be loaded from the PE resource table."
    }
    $bytes = [byte[]]::new([int]$size)
    [Runtime.InteropServices.Marshal]::Copy($pointer, $bytes, 0, [int]$size)
    return $bytes
}

$loadLibraryAsDataFile = 0x00000002
$loadLibraryAsImageResource = 0x00000020
$module = [ZiFile.PackageAudit.NativeResources]::LoadLibraryEx(
    $executable,
    [IntPtr]::Zero,
    $loadLibraryAsDataFile -bor $loadLibraryAsImageResource
)
if ($module -eq [IntPtr]::Zero) {
    $errorCode = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
    throw "Executable could not be loaded for resource audit (Win32 error $errorCode): $executable"
}

try {
    try {
        $groupBytes = Get-ResourceBytes `
            -Module $module `
            -Name 1 `
            -Type 14 `
            -Description 'Reviewed GROUP_ICON resource ID 1'
    }
    catch {
        throw "Executable does not contain reviewed GROUP_ICON resource ID 1: $($_.Exception.Message)"
    }
    if ($groupBytes.Length -lt 6 -or [BitConverter]::ToUInt16($groupBytes, 0) -ne 0 -or
        [BitConverter]::ToUInt16($groupBytes, 2) -ne 1) {
        throw 'Embedded GROUP_ICON resource has an invalid header.'
    }
    $count = [int][BitConverter]::ToUInt16($groupBytes, 4)
    if ($count -ne $ExpectedFrames.Count) {
        throw "Embedded GROUP_ICON must contain exactly $($ExpectedFrames.Count) frames, found $count."
    }
    $directoryEnd = 6 + (14 * $count)
    if ($groupBytes.Length -lt $directoryEnd) {
        throw 'Embedded GROUP_ICON resource has a truncated frame directory.'
    }

    $frameEvidence = @()
    for ($index = 0; $index -lt $count; $index++) {
        $entryOffset = 6 + (14 * $index)
        $width = if ($groupBytes[$entryOffset] -eq 0) { 256 } else { [int]$groupBytes[$entryOffset] }
        $height = if ($groupBytes[$entryOffset + 1] -eq 0) { 256 } else { [int]$groupBytes[$entryOffset + 1] }
        $planes = [BitConverter]::ToUInt16($groupBytes, $entryOffset + 4)
        $bitsPerPixel = [BitConverter]::ToUInt16($groupBytes, $entryOffset + 6)
        $declaredBytes = [BitConverter]::ToUInt32($groupBytes, $entryOffset + 8)
        $resourceId = [BitConverter]::ToUInt16($groupBytes, $entryOffset + 12)
        $expectedSize = $ExpectedFrames[$index]
        if ($width -ne $expectedSize -or $height -ne $expectedSize) {
            throw "Embedded icon frame $index must be ${expectedSize}x${expectedSize}, found ${width}x${height}."
        }
        if ($planes -ne 1 -or $bitsPerPixel -ne 32) {
            throw "Embedded icon frame $expectedSize must use one 32-bit color plane."
        }
        $payload = Get-ResourceBytes `
            -Module $module `
            -Name $resourceId `
            -Type 3 `
            -Description "Embedded ICON resource ID $resourceId"
        if ($payload.Length -ne $declaredBytes -or $payload.Length -lt 24) {
            throw "Embedded icon frame $expectedSize has invalid payload length."
        }
        $pngSignature = [byte[]](0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)
        for ($signatureIndex = 0; $signatureIndex -lt $pngSignature.Count; $signatureIndex++) {
            if ($payload[$signatureIndex] -ne $pngSignature[$signatureIndex]) {
                throw "Embedded icon frame $expectedSize is not PNG encoded."
            }
        }
        $chunkType = [Text.Encoding]::ASCII.GetString($payload, 12, 4)
        $ihdrLength = Read-BigEndianUInt32 -Bytes $payload -Offset 8
        $pngWidth = Read-BigEndianUInt32 -Bytes $payload -Offset 16
        $pngHeight = Read-BigEndianUInt32 -Bytes $payload -Offset 20
        if ($chunkType -cne 'IHDR' -or $ihdrLength -ne 13 -or
            $pngWidth -ne $expectedSize -or $pngHeight -ne $expectedSize) {
            throw "Embedded icon frame $expectedSize has invalid PNG geometry."
        }
        $frameEvidence += [pscustomobject]@{
            size = $expectedSize
            resource_id = $resourceId
            bits_per_pixel = $bitsPerPixel
            encoding = 'png'
            payload_bytes = $payload.Length
        }
    }

    [pscustomobject]@{
        schema_version = 1
        validated = $true
        executable = [IO.Path]::GetFileName($executable)
        group_resource_id = 1
        frame_count = $frameEvidence.Count
        frames = $frameEvidence
    } | ConvertTo-Json -Depth 4
}
finally {
    $null = [ZiFile.PackageAudit.NativeResources]::FreeLibrary($module)
}
