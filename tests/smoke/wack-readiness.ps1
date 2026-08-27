$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$readiness = Join-Path $repoRoot 'packaging\msix\Test-WackReadiness.ps1'
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$testRoot = Join-Path $tempRoot "zifile-wack-readiness-$([Guid]::NewGuid().ToString('N'))"

$tokens = $null
$errors = $null
[Management.Automation.Language.Parser]::ParseFile($readiness, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors.Count -gt 0) { throw "PowerShell parser rejected Test-WackReadiness.ps1: $($errors -join '; ')" }

try {
    New-Item -ItemType Directory -Path $testRoot -ErrorAction Stop | Out-Null
    $package = Join-Path $testRoot 'ZiFile-test-x64.msix'
    Set-Content -LiteralPath $package -Value 'not a signed package' -Encoding utf8
    $auditPath = Join-Path $testRoot 'ZiFile-test-x64.audit.json'
    $formalIdentity = 'ZiCode.ZiFile'
    $formalPublisher = 'CN=ZiCode Official, O=ZiCode'
    $formalPublisherDisplayName = 'ZiCode'
    $audit = [ordered]@{
        schema_version = 2
        package = [IO.Path]::GetFileName($package)
        sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash
        identity = 'ZiCode.ZiFile.Dev'
        publisher = 'CN=ZiCode Development, OID.2.25.311729368913984317654407730594956997722=1'
        publisher_display_name = 'ZiCode Development'
        architecture = 'x64'
        minimum_windows_version = '10.0.26100.0'
        forbidden_file_count = 0
        signature_required = $false
        signature_status = 'NotSigned'
    }
    $audit | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $auditPath -Encoding utf8
    $missingAppCert = Join-Path $testRoot 'missing-appcert.exe'
    $result = & $readiness `
        -PackagePath $package `
        -AuditPath $auditPath `
        -ExpectedIdentityName $formalIdentity `
        -ExpectedPublisher $formalPublisher `
        -ExpectedPublisherDisplayName $formalPublisherDisplayName `
        -AppCertPath $missingAppCert | ConvertFrom-Json
    if ($result.ready) { throw 'Unsigned development WACK fixture was incorrectly marked ready.' }
    $codes = @($result.issues.code)
    foreach ($requiredCode in @(
        'appcert_missing',
        'package_signature_invalid',
        'audit_signature_invalid',
        'identity_mismatch',
        'publisher_mismatch',
        'publisher_display_name_mismatch',
        'unsigned_publisher',
        'minimum_windows_mismatch'
    )) {
        if ($codes -cnotcontains $requiredCode) { throw "WACK readiness omitted expected issue: $requiredCode" }
    }

    $evidencePath = Join-Path $testRoot 'readiness.json'
    try {
        & $readiness `
            -PackagePath $package `
            -AuditPath $auditPath `
            -ExpectedIdentityName $formalIdentity `
            -ExpectedPublisher $formalPublisher `
            -ExpectedPublisherDisplayName $formalPublisherDisplayName `
            -AppCertPath $missingAppCert `
            -EvidencePath $evidencePath `
            -RequireReady | Out-Null
        throw 'RequireReady accepted an unsigned development package.'
    }
    catch {
        if ($_.Exception.Message -notmatch 'WACK readiness failed') { throw }
    }
    $persisted = Get-Content -Raw -LiteralPath $evidencePath | ConvertFrom-Json
    if ($persisted.ready -or $persisted.operation -cne 'wack-readiness') {
        throw 'WACK readiness failure evidence was not persisted correctly.'
    }

    $audit.sha256 = '0' * 64
    $audit | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $auditPath -Encoding utf8
    $tampered = & $readiness `
        -PackagePath $package `
        -AuditPath $auditPath `
        -ExpectedIdentityName $formalIdentity `
        -ExpectedPublisher $formalPublisher `
        -ExpectedPublisherDisplayName $formalPublisherDisplayName `
        -AppCertPath $missingAppCert | ConvertFrom-Json
    if (@($tampered.issues.code) -cnotcontains 'audit_hash_mismatch') {
        throw 'WACK readiness accepted a mismatched package/audit hash.'
    }

    $null = $audit.Remove('publisher_display_name')
    $audit | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $auditPath -Encoding utf8
    $incompleteEvidencePath = Join-Path $testRoot 'incomplete-audit-readiness.json'
    $incomplete = & $readiness `
        -PackagePath $package `
        -AuditPath $auditPath `
        -ExpectedIdentityName $formalIdentity `
        -ExpectedPublisher $formalPublisher `
        -ExpectedPublisherDisplayName $formalPublisherDisplayName `
        -AppCertPath $missingAppCert `
        -EvidencePath $incompleteEvidencePath | ConvertFrom-Json
    if (@($incomplete.issues.code) -cnotcontains 'audit_schema_invalid' -or
        -not (Test-Path -LiteralPath $incompleteEvidencePath -PathType Leaf)) {
        throw 'WACK readiness did not persist structured evidence for an incomplete audit.'
    }

    [pscustomobject]@{
        schema_version = 1
        unsigned_package_rejected = $true
        partner_center_identity_mismatch_rejected = $true
        partner_center_publisher_mismatch_rejected = $true
        partner_center_publisher_display_name_mismatch_rejected = $true
        missing_appcert_reported = $true
        failure_evidence_persisted = $true
        audit_hash_mismatch_rejected = $true
        incomplete_audit_evidence_persisted = $true
    } | ConvertTo-Json
}
finally {
    $resolved = [IO.Path]::GetFullPath($testRoot)
    if (-not $resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.Path]::GetFileName($resolved) -notlike 'zifile-wack-readiness-*') {
        throw "Refusing to remove unexpected WACK test directory: $resolved"
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
