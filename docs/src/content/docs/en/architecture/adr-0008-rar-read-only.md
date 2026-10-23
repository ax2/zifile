---
title: ADR-0008 — RAR read-only boundary
description: The reviewed RAR integration is limited to browsing, integrity testing, and safe extraction.
---

# ADR-0008 — RAR read-only boundary

## Status

Accepted on 2026-10-23. It supersedes the earlier candidate contract that
exposed RAR creation.

## Decision

ZiFile supports RAR 1.3–7 as a beta read-only provider. Supported operations
are listing, integrity testing, password retry, and selective or complete safe
extraction. RAR is excluded from `ArchiveFormat::CREATABLE`, both desktop
create menus, and the CLI creation contract.

## License and security review

- The pinned `rars` 0.9.3 registry package reports `MIT OR Apache-2.0`; its
  repository is `https://github.com/bitplane/rars`.
- `Cargo.lock` pins the registry checksum to
  `68ff174e95890b9e1c01601a93f35f60c017162fb1bd723dfef9f684ae2ffdb6`.
- RAR parsing remains behind the isolated Worker and the core's bounded entry,
  expanded-byte, path-depth, expansion-ratio, cancellation, and
  temporary-output rules.
- RAR-specific tests cover every provider archive version exposed by the
  dependency, encrypted headers, solid selected extraction, malformed and
  truncated input, unsafe links/redirections, cancellation, and output limits.
  The pinned external corpus and fuzz target remain required CI evidence.
- Creation is intentionally excluded because producing RAR is not a required
  open-format capability for ZiFile and would expand the licensing,
  compatibility, and support surface without improving the read safety case.

## Consequences

The format matrix reports `CREATE=no`, `CREATE_INPUT=none`, and
`COMPRESSION_LEVEL=none` for RAR. Existing RAR files remain associated with
ZiFile and can be opened, tested, and extracted. Users who need to create RAR
files must use a separately licensed RAR writer.

