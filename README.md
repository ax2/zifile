# ZiFile

ZiFile is a modern, open-source archive and file utility for Windows. It is
being written from scratch with Rust, including the desktop UI, and is licensed
under MIT.

The project is currently in **Stage 4 — public 0.1 release and 1.0 readiness**. ZIP, encrypted
ZIP, 7z, TAR compositions (including TAR + LZMA and TAR + LZ4), standalone LZMA, and common compression streams have working create,
list, integrity-test and safe-extraction paths shared by the CLI and Iced
desktop application. RAR 1.3–7 and Windows CAB are available as pure-Rust,
read-only beta providers.

The current build is feature-complete for the currently supported archive contract:
the source, x64/ARM64 builds, package audits, reproducibility checks and release
rehearsals are automated. The public [`v0.1.14` GitHub release](https://github.com/ax2/zifile/releases/tag/v0.1.14)
uses unsigned artifacts; a stable 1.0 release is still gated on
trusted signing, real foreground Windows validation, physical ARM64, WACK,
Partner Center/Microsoft Store certification and WinGet acceptance.

The desktop UI is available in Simplified Chinese and English, follows the
system language on first launch, and persists the selected language, theme, and
up to eight successfully opened local archive paths. Recent history can be
reopened, removed one item at a time, or cleared from the home page; passwords
and archive contents are never persisted. Archive rows are searchable and paged in bounded groups of 500 so very
large listings do not create an unbounded widget tree.

For ZIP, 7z, and TAR-family archives, entries can be added, removed, or renamed
through an atomic sibling-staging rebuild. The desktop surfaces expose a
single-entry and batch rename editors, while the CLI supports repeated mappings such as
`zifile rename archive.zip --rename old.txt=new.txt`.

The packaged Windows 11 Explorer integration adds a create command for selected
files, selected folders, and a folder's background, plus an extract command for
supported archive files. The extension only forwards local paths to the desktop;
archive work remains in the isolated Worker.

Desktop drag-and-drop probes file signatures before falling back to a known
extension hint, so a valid archive can still be opened after being renamed while
ordinary files remain creation sources. The small probe runs outside the UI event
thread, and the isolated Worker still performs the definitive archive listing.

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
cargo run -p zifile-cli -- rename archive.zip --rename old.txt=new.txt
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
- [Code signing policy](CODE-SIGNING-POLICY.md)
- [1.0 release readiness manifest](release/readiness.json)
- [Stage 0 work log](docs/src/content/docs/releases/stage-0.md)
- [Stage 1 work log](docs/src/content/docs/releases/stage-1.md)
- [Stage 2 work log](docs/src/content/docs/releases/stage-2.md)
- [Stage 3 work log](docs/src/content/docs/releases/stage-3.md)
- [Stage 4 work log](docs/src/content/docs/releases/stage-4.md)

## Status

| Area | Current state |
| --- | --- |
| Core model | Real create/list/test/extract operations with shared safety policy |
| Desktop | Bilingual modern browser/creator, isolated archive worker and Windows taskbar progress |
| CLI | `formats`, `detect`, `list`, `test`, `extract`, `create`, `update`, and safe in-place `rename` |
| Archive providers | ZIP/ZIP64/AES, 7z/AES, read-only RAR 1.3 through RAR 7 with encryption, read-only Windows CAB, TAR compositions including TAR + LZMA and TAR + LZ4, standalone LZMA, and common streams |
| Packaging | Real x64/ARM64 build outputs combined into one all-in-one MSIX bundle for users |
| Distribution | Tag workflow publishes one MSIX bundle, one standalone portable EXE per architecture and one checksum; SBOM, provenance, audits and WinGet candidates remain build evidence |

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
certification and Store submission are still in progress, so no trusted
production package release has been published yet.

## Code signing policy

Free code signing provided by SignPath.io, certificate by SignPath Foundation.
The application is being prepared and is not yet approved; current release
artifacts must not be described as SignPath Foundation signed. See the
[Code signing policy](CODE-SIGNING-POLICY.md) for roles, provenance, privacy,
and the separation between WinGet and the Partner Center Store identity.
