# Microsoft Store assets

The bilingual listing JSON is authoritative for Partner Center copy. Run the
policy gate before publishing:

```powershell
./packaging/store/Test-Listings.ps1
./packaging/store/Test-Screenshots.ps1
```

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
