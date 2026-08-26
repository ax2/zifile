# ZiFile

ZiFile is a modern, open-source archive and file utility for Windows. It is
being written from scratch with Rust, including the desktop UI, and is licensed
under MIT.

The project is currently in **Stage 1 — Alpha development**. ZIP, encrypted
ZIP, 7z, TAR compositions and common compression streams have working create,
list, integrity-test and safe-extraction paths shared by the CLI and Iced
desktop application. RAR 1.3–7 and Windows CAB are available as pure-Rust,
read-only beta providers.

The desktop UI is available in Simplified Chinese and English, follows the
system language on first launch, and persists only the selected language and
theme. Archive rows are searchable and paged in bounded groups of 500 so very
large listings do not create an unbounded widget tree.

An opt-in Dioxus/WebView2 accessibility candidate now exercises the same isolated
Worker through semantic navigation, archive, integrity-test, extraction and creation
screens. It is not yet the packaged default: Narrator/Accessibility Insights,
high-contrast, IME, DPI and ARM64 runtime gates remain open. The candidate now has
local-only WebView resources, native drop handling, core shortcuts, a locally exercised
x64 package and cloud-verified x64/ARM64 MSIX and executable artifacts.

## Project identity

- Product: **ZiFile**
- Publisher: **ZiCode**
- Documentation: <https://ax2.github.io/zifile/>
- Planned product domain: `zifile.zicode.com`
- Repository: <https://github.com/ax2/zifile>
- WinGet package ID: `ZiCode.ZiFile` (planned)
- License: [MIT](LICENSE)

## Run ZiFile

```powershell
cargo test --workspace
cargo run -p zifile-cli -- formats
cargo run -p zifile-cli -- list archive.zip
cargo run -p zifile-cli -- extract archive.zip output --conflict rename
cargo run -p zifile-cli -- create output.7z files --format seven-zip
cargo run -p zifile-desktop
```

Build the documentation:

```powershell
pnpm --dir docs install
pnpm --dir docs build
```

## Documentation

- [Getting started](docs/src/content/docs/en/guides/getting-started.md)
- [Troubleshooting](docs/src/content/docs/en/guides/troubleshooting.md)
- [Product vision](docs/src/content/docs/product/vision.md)
- [Roadmap](ROADMAP.md)
- [Architecture](docs/src/content/docs/architecture/overview.md)
- [Security model](docs/src/content/docs/architecture/security.md)
- [Security policy and private reporting](SECURITY.md)
- [Testing strategy](docs/src/content/docs/development/testing.md)
- [Contributing](CONTRIBUTING.md)
- [Desktop usage and accessibility](docs/src/content/docs/development/desktop.md)
- [Release process](docs/src/content/docs/development/releasing.md)
- [Production signing operations](docs/src/content/docs/development/signing-operations.md)
- [1.0 release readiness manifest](release/readiness.json)
- [Stage 0 work log](docs/src/content/docs/releases/stage-0.md)
- [Stage 1 work log](docs/src/content/docs/releases/stage-1.md)

## Status

| Area | Current state |
| --- | --- |
| Core model | Real create/list/test/extract operations with shared safety policy |
| Desktop | Bilingual modern browser/creator, isolated archive worker and Windows taskbar progress |
| CLI | `formats`, `detect`, `list`, `test`, `extract`, and `create` |
| Archive providers | ZIP/ZIP64/AES, 7z/AES, read-only RAR 1.3 through RAR 7 with encryption, read-only Windows CAB, TAR compositions, and common streams |
| Packaging | Real x64/ARM64 runnable directory, EXE, MSIX, CLI alias and audited Rust shell DLL |
| Distribution | Tag workflow produces checksums, SBOM, provenance and WinGet manifest candidates |

Encrypted CLI operations read one password line from standard input. ZiFile does
not accept a plaintext password argument, keeping it out of the process command
line and ordinary shell history:

```powershell
$password | zifile test archive.7z --password-stdin
$password | zifile extract archive.7z output --password-stdin
```

RAR creation remains disabled; browsing, testing and selective extraction use the
pure-Rust permissively licensed `rars` provider behind ZiFile's safety and Worker-isolation
boundaries. Trusted-package shell activation, signing, accessibility
certification and Store submission are still in
progress, so no production release has been tagged yet.
