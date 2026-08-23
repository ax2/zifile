param(
    [switch]$SkipDesktopLaunch
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Push-Location $repoRoot

try {
    cargo build --workspace --locked

    $formats = (cargo run --quiet -p zifile-cli -- formats) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $formats -notmatch 'ZIP' -or $formats -notmatch '7z') {
        throw 'CLI format registry smoke test failed.'
    }

    $detection = (cargo run --quiet -p zifile-cli -- detect sample.tar.gz) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $detection -notmatch 'TAR \+ gzip') {
        throw 'CLI format detection smoke test failed.'
    }

    $desktopPath = Join-Path $repoRoot 'target\debug\zifile-desktop.exe'
    if (-not (Test-Path -LiteralPath $desktopPath)) {
        throw 'Desktop executable was not produced.'
    }

    if (-not $SkipDesktopLaunch) {
        $process = Start-Process -FilePath $desktopPath -PassThru
        Start-Sleep -Seconds 3
        if ($process.HasExited) {
            throw "Desktop smoke test exited unexpectedly with code $($process.ExitCode)."
        }
        Stop-Process -Id $process.Id
    }

    Write-Host 'ZiFile foundation smoke test passed.'
}
finally {
    Pop-Location
}
