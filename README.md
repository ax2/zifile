# ZiFile

ZiFile is a modern, open-source archive and file utility for Windows. It is
being written from scratch with Rust, including the desktop UI, and is licensed
under MIT.

The project is currently in **Stage 0 — Foundation**. The repository contains a
working Rust workspace, a small Iced desktop shell, a CLI format registry,
tests, a benchmark, CI, and the first complete set of product and architecture
documents. Archive I/O begins in Stage 1.

## Project identity

- Product: **ZiFile**
- Publisher: **ZiCode**
- Documentation: <https://ax2.github.io/zifile/>
- Planned product domain: `zifile.zicode.com`
- Repository: <https://github.com/ax2/zifile>
- WinGet package ID: `ZiCode.ZiFile` (planned)
- License: [MIT](LICENSE)

## Run the current foundation

```powershell
cargo test --workspace
cargo run -p zifile-cli -- formats
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
- [Release process](docs/src/content/docs/development/releasing.md)
- [Stage 0 work log](docs/src/content/docs/releases/stage-0.md)

## Status

| Area | Current state |
| --- | --- |
| Core model | Format registry and safety limits implemented |
| Desktop | Iced technology shell implemented |
| CLI | `formats` and extension-based `detect` commands implemented |
| Archive providers | Planned for Stage 1 and Stage 2 |
| Packaging | Structure reserved; MSIX/MSI work begins in Stage 2 |
| Distribution | GitHub Actions foundation present; WinGet and Store planned |

ZiFile does not currently extract or create archives. The UI labels and CLI
output intentionally describe roadmap capability rather than claiming shipped
support.
