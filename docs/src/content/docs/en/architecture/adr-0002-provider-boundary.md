---
title: "ADR-0002: Format Provider boundary"
description: Isolating archive implementations behind capability-driven interfaces.
---

- Status: accepted
- Date: 2026-08-23

## Decision

Every archive format is integrated through one Provider boundary. A Provider explicitly reports its browse, extract, create, test, encryption, and multipart capabilities.

## Consequences

- The UI does not depend directly on compression crates.
- The CLI and desktop share capability and error models.
- A backend with security or maintenance issues can be replaced.
- Formats with separate compatibility or licensing questions, including RAR, remain isolated for review.
- Benchmarks, interoperability tests, and fuzzing can run per Provider.
