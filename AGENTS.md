# ZiFile repository instructions

## Product boundary

- ZiFile is an MIT-licensed Windows archive and future file utility written primarily in Rust, including its UI.
- Do not copy or fork implementation code from 7-Zip, PeaZip, ouch, or other archive applications.
- Do not claim a format or operation is supported until an implementation and interoperability test prove it.
- RAR creation is out of scope. Any RAR read support requires an explicit license and security review.

## Documentation and tracking

- Keep `README.md`, `ROADMAP.md`, `CHANGELOG.md`, and the Starlight site synchronized with behavior and release status.
- Every Stage has a page under `docs/src/content/docs/releases/` recording goals, findings, changes, verification, remaining work, and release result.
- Architecture changes require an ADR under `docs/src/content/docs/architecture/`.
- Missing historical evidence must be marked as missing; never reconstruct fictional work history.

## Engineering gates

- Run formatting, Clippy with warnings denied, workspace tests, the foundation smoke test, and the Starlight build before release.
- Archive parsing is hostile-input handling. New providers require security fixtures, interoperability tests, fuzz targets, and bounded resource behavior.
- Keep UI work off the archive worker thread and make long tasks observable and cancelable.
- Approved dependency licenses are enforced by `cargo-deny`; do not add restricted, unknown, or incompatible code silently.

## Releases

- Version all crates, docs, packages, and manifests from one release decision.
- Public release artifacts require checksums, provenance, an SBOM, and a corresponding Stage log update.
- Never commit certificates, passwords, private keys, cookies, tokens, Partner Center credentials, or real customer archives.
