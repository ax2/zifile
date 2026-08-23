# WinGet packaging

Planned package ID: `ZiCode.ZiFile`.

The first manifest will be generated only after a signed, versioned MSI or MSIX
is published at a stable HTTPS URL. CI will validate the manifest locally and a
release workflow will prepare an update pull request for
`microsoft/winget-pkgs`.

No placeholder manifest is committed because an installable manifest must
contain real version, hash, installer type, architecture, and URL values.
