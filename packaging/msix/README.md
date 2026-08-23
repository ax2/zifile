# MSIX packaging

MSIX is the primary Microsoft Store package target. `Build-Package.ps1` creates
both a complete runnable Windows directory and an MSIX without making a ZIP
archive. The package is unsigned unless a PFX and secure password are supplied.

```powershell
./packaging/msix/Build-Package.ps1 -Version 0.1.0.0 -Architecture x64
```

`AppxManifest.xml` uses a development identity. A Store build must pass the
Partner Center identity name and publisher to the script. Direct distribution
may additionally pass a PFX path and secure password; credentials and
certificates must never be committed.

Implemented packaging features:

- x64 and ARM64 builds from one manifest;
- archive file associations and full-trust desktop capability;
- deterministic app icon/tile assets;
- optional Authenticode signing and SHA-256 generation;
- complete runnable directories and standalone EXEs without ZIP output.

Before Store submission the project still needs a Partner Center identity,
trusted signing credentials for direct distribution, upgrade/uninstall test
evidence, Store listing screenshots, and Windows Application Certification Kit
verification.

Do not commit certificates, private keys, Partner Center secrets, or tenant
credentials to this directory.
