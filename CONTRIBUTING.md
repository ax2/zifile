# Contributing to ZiFile

ZiFile accepts focused issues and pull requests. Before making a large change,
open a design issue so format behavior, security impact, and user experience can
be agreed before implementation.

## Local checks

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir docs install --frozen-lockfile
pnpm --dir docs build
```

Behavior changes must update tests and the relevant documentation. Architecture
changes require an ADR under `docs/src/content/docs/architecture/`.

All contributions are accepted under the repository's MIT license.
