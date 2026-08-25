## Summary

Describe the user-visible outcome, compatibility impact, and evidence boundary.

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-targets --all-features --locked`
- [ ] `./tests/smoke/foundation.ps1 -SkipDesktopLaunch`
- [ ] `./tests/smoke/packaging-policy.ps1`
- [ ] `pnpm --dir docs build`
- [ ] Documentation updated for behavior changes
- [ ] `CHANGELOG.md` updated for notable changes
- [ ] Both Rust desktop UIs considered for shared behavior changes
- [ ] Security and license impact considered
- [ ] No external certification or release claim exceeds the attached evidence
