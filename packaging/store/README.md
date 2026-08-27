# Microsoft Store assets

`listing-assets/AppTile300x300.png` is the reviewed 1:1 app tile icon recommended
for the Partner Center listing. `listing-assets.json` pins its purpose, exact
dimensions, Microsoft requirements source, and SHA-256. CI runs
`Test-ListingAssets.ps1` and negative smoke fixtures so a missing, resized, or
modified icon cannot silently reach submission. This directory is separate from
the signed-candidate screenshot `assets/` tree below.

Formal Store and trusted-signing builds use three non-secret repository
variables copied exactly from Partner Center: `ZIFILE_MSIX_IDENTITY`,
`ZIFILE_MSIX_PUBLISHER`, and `ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME`. The last
value is the developer account's Publisher Display Name, not the app's reserved
product name. `Test-PartnerCenterIdentity.ps1 -RequireConfigured` rejects a
partial tuple before compilation, and the package audit verifies all three
values after unpacking the MSIX.

The bilingual listing JSON is authoritative for Partner Center copy. Run the
policy gate before publishing:

```powershell
./packaging/store/Test-Listings.ps1
./packaging/store/Test-Screenshots.ps1
./packaging/store/Test-PublicPrivacy.ps1 -DocumentationOutput ./docs/dist
```

The two listing privacy URLs are fixed to the localized GitHub Pages routes.
Normal CI verifies their generated `index.html` files and privacy markers; the
Pages workflow repeats the check against the deployed HTTPS pages after publish.
The live check may retry propagation, but requires HTTP 200 and real policy copy.

Formal screenshot capture input must contain exactly these files:

```text
capture/
  zh-CN/01-home.png
  zh-CN/02-create.png
  zh-CN/03-browse.png
  zh-CN/04-extract.png
  en-US/01-home.png
  en-US/02-create.png
  en-US/03-browse.png
  en-US/04-extract.png
```

Import images only from one signed candidate and record its actual metadata:

```powershell
./packaging/store/Import-Screenshots.ps1 `
  -SourceDirectory C:\capture `
  -SourceCommit 0123456789abcdef0123456789abcdef01234567 `
  -AppVersion 1.0.0 `
  -WindowsBuild 10.0.26100 `
  -Theme light `
  -ScalePercent 100 `
  -CapturedAtUtc 2026-08-25T15:00:00Z `
  -CandidateKind signed-msix
```

The importer requires the repository manifest to remain `draft` and refuses an
existing `assets` directory. It stages all eight PNGs, writes hashes and fixed
bilingual captions, validates the complete manifest, then moves the validated
asset tree into place. It never overwrites an earlier formal set.

Tagged publishing requires the resulting complete manifest. Test-generated
images and unsigned development screenshots are not formal Store assets.
