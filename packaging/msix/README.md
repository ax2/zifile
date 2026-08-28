# MSIX packaging

`Test-Assets.ps1` validates the complete reviewed Windows icon matrix: 58 PNGs,
including 11 scale-qualified package assets and all 14 app-list target sizes in
default, dark-unplated, and light-unplated forms. It also checks exact dimensions,
alpha, a five-frame 16/24/32/48/256 Win32 desktop icon, all required manifest logo
references, and the pinned SHA-256 catalog in `assets.json`. The icon gate parses
ICONDIR entries, payload bounds, PNG signatures, IHDR dimensions, color planes, and
bit depth instead of trusting only the largest frame exposed by a platform decoder.
x64 CI additionally checks byte-for-byte output
from `Generate-Assets.ps1`; package builds intentionally skip that host-specific
redraw because System.Drawing rasterization can differ across CPU architectures.
CI and `Build-Package.ps1` run the portable gate before packaging, and
`Test-Package.ps1` verifies all 58 reviewed PNG hashes and the desktop ICO hash again
after unpacking the built MSIX. `Test-EmbeddedIcon.ps1` then loads the packaged desktop
PE as a data/image resource, requires `GROUP_ICON` ID 1 plus five `ICON` resources,
and validates each resource's size, PNG signature, IHDR geometry, plane count, and bit
depth. This runs for both x64 and ARM64 package jobs without executing the target PE,
so a stale, missing, substituted, or unembedded icon cannot enter a candidate.

MSIX is the primary Microsoft Store package target. `Build-Package.ps1` creates
both a complete runnable Windows directory and an MSIX without making a ZIP
archive. The builder is unsigned by default. Its local PFX parameters remain
only for isolated developer experiments; GitHub Release never consumes a PFX
and signs staged artifacts through a cloud-HSM provider.

```powershell
./packaging/msix/Build-Package.ps1 -Version 0.1.0.0 -Architecture x64
```

From an ordinary PowerShell session the script locates `VsDevCmd.bat` when `cl.exe` is not
already available. ARM64 cross-compilation additionally checks for the Visual Studio
**MSVC ARM64/ARM64EC build tools** component and fails before Cargo with an actionable error
when its compiler or runtime libraries are absent.

`AppxManifest.xml` uses a development identity in the Windows 11 unsigned-package
OID namespace. Microsoft documents `Add-AppxPackage -AllowUnsigned` from an
elevated shell for this path; this identity can never collide with a signed
package. Windows only supports this unsigned executable-package path on Windows
11, so development packages target build 26100 or newer. The current build 26200
test machine still rejected the OID publisher with deployment error `0x80080204`,
so local unsigned installation is not yet a passed gate. Signed and
Store packages retain the manifest's Windows 10 build 19041 minimum. A Store
build must pass the Partner Center identity name, publisher, and exact publisher
display name to the script.
Credentials and certificates must never be committed.

When `CertificatePath` is supplied, `Publisher` must be the exact certificate
subject and must not contain the unsigned OID. An unsigned non-development Store
submission may use the Partner Center publisher because Microsoft signs accepted
Store packages.

Implemented packaging features:

- x64 and ARM64 builds from one manifest;
- archive file associations and full-trust desktop capability;
- a packaged `zifile.exe` App Execution Alias for terminal and automation use;
- deterministic high-DPI app icon/tile assets for Store, Start, Search, taskbar,
  context-menu, title-bar, and light/dark shell surfaces;
- optional Authenticode signing and SHA-256 generation;
- automatic post-build audit of package identity, publisher, publisher display name, version, minimum Windows build,
  PE architecture, required executables, file associations, CLI alias, forbidden sensitive files,
  and required signature status;
- a machine-readable `.audit.json` beside every MSIX, included in release checksums and provenance;
- complete runnable directories and standalone EXEs without ZIP output;
- the architecture-matched `zifile-worker.exe` beside the desktop executable and inside MSIX.
- an architecture-matched Rust `IExplorerCommand` COM DLL registered for the Windows 11 modern
  File Explorer menu, with its CLSID, item types and PE machine covered by the package audit.

The desktop executable requires its matching Worker. GitHub release staging therefore publishes
both architecture-suffixed files; complete runnable directories retain the canonical sibling name.

Tagged GitHub releases build with formal `ZIFILE_MSIX_IDENTITY` and
`ZIFILE_MSIX_PUBLISHER` plus `ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME` environment
variables, then enter the protected
`production-signing` Environment. DigiCert Binary Signing signs the staged EXEs,
DLL, and MSIX with the cloud-held key. `Test-SignedReleaseArtifacts.ps1` requires
valid signatures, one exact Publisher, and timestamps before regenerating audits
and checksums; publishing downloads only `signed-windows-*`. Manual workflow runs
may select `none` for unsigned validation or `digicert-stm` for a protected rehearsal.
Provisioning, approval, credential rotation, emergency stop, revocation, and evidence retention are
defined in `docs/src/content/docs/development/signing-operations.md` and machine-checked by
`scripts/Test-SigningOperationsDocs.ps1`.

The MSIX also registers `zifile.exe` as an App Execution Alias. Users can disable aliases in
Windows Settings, so automation must not assume that the alias is always enabled. The packaged
shell DLL exposes “Create archive with ZiFile” for files and directories, then launches the
desktop create page with the selected paths. A second command is visible for one supported archive
and launches `--extract-here`; after signature-first listing, the desktop extracts all regular files
to a sibling folder matching the archive stem with rename-on-conflict behavior. Encrypted archives
remain in the visible password retry flow. The DLL never performs archive work or handles passwords
inside Explorer.
Real menu activation remains an install-time gate because the current development package cannot
be registered on this machine.

Trusted signed baseline and upgrade packages can be exercised on a clean test account with:

```powershell
./tests/smoke/msix-lifecycle.ps1 `
  -BaselinePackage ./ZiFile-1.0.0.0.msix `
  -UpgradePackage ./ZiFile-1.0.1.0.msix `
  -Architecture x64 `
  -BaselineVersion 1.0.0.0 `
  -UpgradeVersion 1.0.1.0 `
  -IdentityName ZiCode.ZiFile `
  -Publisher 'CN=ZiCode Official' `
  -PublisherDisplayName 'ZiCode' `
  -MinimumWindowsVersion 10.0.19041.0 `
  -ConfirmLifecycle
```

The gate requires valid trusted signatures, refuses to touch any existing package with the same
identity, verifies the installed CLI, upgrades in place, runs the supported `Reset-AppxPackage`
recovery operation, and uninstalls in `finally`. Reset restores initial configuration and is
recorded separately; it is not claimed as a data-preserving Repair operation.

After two manual Release workflow runs have produced `signed-windows-x64` artifacts, run
**Trusted MSIX lifecycle** with their baseline and upgrade run IDs. The workflow downloads both
artifacts into a clean Windows Runner, derives package metadata from their audit JSON, executes the
same lifecycle gate, and retains evidence for 30 days. ARM64 package installation still requires a
physical ARM64 Windows test environment.

Before launching WACK, evaluate the exact package/audit pair without installing or executing it:

```powershell
./packaging/msix/Test-WackReadiness.ps1 `
  -PackagePath ./ZiFile-1.0.0.0-windows-x64.msix `
  -AuditPath ./ZiFile-1.0.0.0-windows-x64.audit.json `
  -ExpectedIdentityName $env:ZIFILE_MSIX_IDENTITY `
  -ExpectedPublisher $env:ZIFILE_MSIX_PUBLISHER `
  -ExpectedPublisherDisplayName $env:ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME `
  -EvidencePath ./wack-readiness-x64.json `
  -RequireReady
```

The readiness gate requires an interactive administrator session, an installed Windows App
Certification Kit, matching host/package architecture, a matching schema-v2 audit and package
SHA-256, the exact formal Partner Center Identity/Publisher/Publisher Display Name tuple,
Windows 10 build 19041 minimum support, zero forbidden files,
and a trusted `Valid` signature in both the package and its audit. It does not run WACK, install the
package, or replace the certification report.

Before Store submission the project still needs a Partner Center identity,
trusted signing credentials for direct distribution, upgrade/uninstall test
evidence, Store listing screenshots, and Windows Application Certification Kit
verification.

Do not commit certificates, private keys, Partner Center secrets, or tenant
credentials to this directory.
