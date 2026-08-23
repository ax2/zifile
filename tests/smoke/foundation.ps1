param(
    [switch]$SkipDesktopLaunch
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Push-Location $repoRoot

try {
    cargo build --workspace --locked
    if ($LASTEXITCODE -ne 0) {
        throw 'Workspace build failed.'
    }

    $formats = (cargo run --quiet -p zifile-cli -- formats) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $formats -notmatch 'ZIP' -or $formats -notmatch '7z') {
        throw 'CLI format registry smoke test failed.'
    }

    $tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    $smokeRoot = Join-Path $tempRoot ("zifile-smoke-" + [guid]::NewGuid().ToString('N'))
    $smokeRoot = [System.IO.Path]::GetFullPath($smokeRoot)
    if (-not $smokeRoot.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to create a smoke directory outside the system temporary directory.'
    }
    New-Item -ItemType Directory -Path (Join-Path $smokeRoot 'input\nested') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $smokeRoot 'input\hello.txt') -Value 'hello ZiFile' -NoNewline
    Set-Content -LiteralPath (Join-Path $smokeRoot 'input\nested\unicode-测试.txt') -Value 'archive smoke' -NoNewline

    $archivePath = Join-Path $smokeRoot 'smoke.tar.gz'
    cargo run --quiet -p zifile-cli -- create $archivePath (Join-Path $smokeRoot 'input') --format tar-gzip
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $archivePath)) {
        throw 'CLI archive creation smoke test failed.'
    }

    $detection = (cargo run --quiet -p zifile-cli -- detect $archivePath) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $detection -notmatch 'TAR \+ gzip') {
        throw 'CLI signature detection smoke test failed.'
    }

    $listing = (cargo run --quiet -p zifile-cli -- list $archivePath) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $listing -notmatch 'hello.txt' -or $listing -notmatch 'unicode-测试.txt') {
        throw 'CLI archive listing smoke test failed.'
    }

    cargo run --quiet -p zifile-cli -- test $archivePath
    if ($LASTEXITCODE -ne 0) {
        throw 'CLI archive integrity smoke test failed.'
    }

    $extractPath = Join-Path $smokeRoot 'output'
    cargo run --quiet -p zifile-cli -- extract $archivePath $extractPath
    if ($LASTEXITCODE -ne 0 -or
        (Get-Content -Raw -LiteralPath (Join-Path $extractPath 'input\hello.txt')) -ne 'hello ZiFile') {
        throw 'CLI safe extraction smoke test failed.'
    }

    $desktopPath = Join-Path $repoRoot 'target\debug\zifile-desktop.exe'
    if (-not (Test-Path -LiteralPath $desktopPath)) {
        throw 'Desktop executable was not produced.'
    }

    if (-not $SkipDesktopLaunch) {
        $process = Start-Process -FilePath $desktopPath -PassThru -WindowStyle Hidden
        Start-Sleep -Seconds 3
        if ($process.HasExited) {
            throw "Desktop smoke test exited unexpectedly with code $($process.ExitCode)."
        }
        Stop-Process -Id $process.Id
    }

    Write-Host 'ZiFile archive and desktop smoke test passed.'
}
finally {
    if ($smokeRoot -and (Test-Path -LiteralPath $smokeRoot)) {
        $resolvedSmokeRoot = [System.IO.Path]::GetFullPath($smokeRoot)
        $resolvedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
        if ($resolvedSmokeRoot.StartsWith($resolvedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolvedSmokeRoot -Recurse -Force
        }
    }
    Pop-Location
}
