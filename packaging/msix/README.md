# MSIX packaging

MSIX is the primary Microsoft Store package target. The package identity will
be added after `ZiFile` is reserved in Partner Center.

Stage 2 must add:

- x64 and ARM64 package manifests;
- file associations and declared capabilities;
- visual assets and Store listing assets;
- upgrade and uninstall tests;
- Windows Application Certification Kit verification;
- signed direct-distribution package and Store upload package.

Do not commit certificates, private keys, Partner Center secrets, or tenant
credentials to this directory.
