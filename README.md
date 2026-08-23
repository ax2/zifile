# ZiFile

ZiFile is a modern, open-source archive and file utility for Windows. It is
being written from scratch with Rust, including the desktop UI, and is licensed
under MIT.

The project is currently in **Stage 1 — Alpha development**. ZIP, encrypted
ZIP, 7z, TAR compositions and common compression streams have working create,
list, integrity-test and safe-extraction paths shared by the CLI and Iced
desktop application.

The desktop UI is available in Simplified Chinese and English, follows the
system language on first launch, and persists only the selected language and
theme. Archive rows are searchable and paged in bounded groups of 500 so very
large listings do not create an unbounded widget tree.

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

- [Product vision](docs/src/content/docs/product/vision.md)
- [Roadmap](ROADMAP.md)
- [Architecture](docs/src/content/docs/architecture/overview.md)
- [Security model](docs/src/content/docs/architecture/security.md)
- [Testing strategy](docs/src/content/docs/development/testing.md)
- [Desktop usage and accessibility](docs/src/content/docs/development/desktop.md)
- [Release process](docs/src/content/docs/development/releasing.md)
- [Stage 0 work log](docs/src/content/docs/releases/stage-0.md)
- [Stage 1 work log](docs/src/content/docs/releases/stage-1.md)

## Status

| Area | Current state |
| --- | --- |
| Core model | Real create/list/test/extract operations with shared safety policy |
| Desktop | Bilingual modern browser/creator backed by an isolated, cancelable archive worker |
| CLI | `formats`, `detect`, `list`, `test`, `extract`, and `create` |
| Archive providers | ZIP/ZIP64/AES, 7z/AES, TAR + gzip/zstd/xz/bzip2, and common streams |
| Packaging | Real x64/ARM64 runnable directory, standalone EXE and MSIX build path |
| Distribution | Tag workflow produces checksums, SBOM, provenance and WinGet manifest candidates |

RAR remains disabled pending the explicit license, security and interoperability
review required by this repository. Signing, shell integration, accessibility
certification and Store submission are still in
progress, so no production release has been tagged yet.
