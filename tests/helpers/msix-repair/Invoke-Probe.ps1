param(
    [Parameter(Mandatory)]
    [string]$HelperPath,
    [ValidateRange(1, 120)]
    [int]$TimeoutSeconds = 15,
    [switch]$TimeoutFixture
)

$ErrorActionPreference = 'Stop'
$helper = (Resolve-Path -LiteralPath $HelperPath).Path
$childArgument = if ($TimeoutFixture) { '--probe-child-hang' } else { '--probe' }
$effectiveTimeout = if ($TimeoutFixture) { 1 } else { $TimeoutSeconds }

$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $helper
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true
$startInfo.RedirectStandardOutput = $true
$startInfo.RedirectStandardError = $true
[void]$startInfo.ArgumentList.Add($childArgument)

$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $startInfo
try {
    if (-not $process.Start()) {
        throw 'Cannot start the isolated MSIX Repair probe.'
    }
    $outputTask = $process.StandardOutput.ReadToEndAsync()
    $errorTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($effectiveTimeout * 1000)) {
        try {
            $process.Kill()
        }
        catch [System.InvalidOperationException] {
            # The process exited between the timeout and termination request.
        }
        [ordered]@{
            schema_version = 1
            operation = 'probe'
            probe_completed = $false
            probe_timed_out = $true
            repair_supported = $false
            repair_semantics = 'preserve_application_data'
        } | ConvertTo-Json
        return
    }

    $output = $outputTask.GetAwaiter().GetResult()
    $stderrText = $errorTask.GetAwaiter().GetResult()
    if ($process.ExitCode -ne 0) {
        throw "MSIX Repair probe exited with code $($process.ExitCode): $stderrText"
    }
    Write-Output $output
}
finally {
    $process.Dispose()
}
