# MSIX packaging

MSIX is the primary Microsoft Store package target. `Build-Package.ps1` creates
both a complete runnable Windows directory and an MSIX without making a ZIP
archive. The package is unsigned unless a PFX and secure password are supplied.

```powershell
./packaging/msix/Build-Package.ps1 -Version 0.1.0.0 -Architecture x64
```

`AppxManifest.xml` uses a development identity in the Windows 11 unsigned-package
OID namespace. Microsoft documents `Add-AppxPackage -AllowUnsigned` from an
elevated shell for this path; this identity can never collide with a signed
package. Windows only supports this unsigned executable-package path on Windows
11, so development packages target build 26100 or newer. The current build 26200
test machine still rejected the OID publisher with deployment error `0x80080204`,
so local unsigned installation is not yet a passed gate. Signed and
Store packages retain the manifest's Windows 10 build 19041 minimum. A Store
build must pass the Partner Center identity name and
publisher to the script. Direct distribution may additionally pass a PFX path
and secure password; credentials and certificates must never be committed.

When `CertificatePath` is supplied, `Publisher` must be the exact certificate
subject and must not contain the unsigned OID. An unsigned non-development Store
submission may use the Partner Center publisher because Microsoft signs accepted
Store packages.

Implemented packaging features:

- x64 and ARM64 builds from one manifest;
- archive file associations and full-trust desktop capability;
- a packaged `zifile.exe` App Execution Alias for terminal and automation use;
- deterministic app icon/tile assets;
- optional Authenticode signing and SHA-256 generation;
- complete runnable directories and standalone EXEs without ZIP output;
- the architecture-matched `zifile-worker.exe` beside the desktop executable and inside MSIX.

The desktop executable requires its matching Worker. GitHub release staging therefore publishes
both architecture-suffixed files; complete runnable directories retain the canonical sibling name.

The MSIX also registers `zifile.exe` as an App Execution Alias. Users can disable aliases in
Windows Settings, so automation must not assume that the alias is always enabled. File Explorer
context menus are not registered yet: the modern Windows 11 path requires a separately
implemented and packaged `IExplorerCommand` COM DLL.

Before Store submission the project still needs a Partner Center identity,
trusted signing credentials for direct distribution, upgrade/uninstall test
evidence, Store listing screenshots, and Windows Application Certification Kit
verification.

Do not commit certificates, private keys, Partner Center secrets, or tenant
credentials to this directory.
