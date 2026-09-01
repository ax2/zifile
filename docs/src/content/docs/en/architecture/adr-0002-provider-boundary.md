---
title: "ADR-0002: Format Provider boundary"
description: Isolating archive implementations behind capability-driven interfaces.
---

- Status: accepted
- Date: 2026-08-23

## Decision

Every archive format is integrated through one Provider boundary. A Provider explicitly reports its browse, extract, create, test, encryption, multipart, and create-input-shape capabilities. Create input distinguishes files-and-directories from exactly one file; read-only formats such as RAR report that creation is unavailable, while CAB reports fixed-MSZIP creation without encryption.

## Consequences

- The UI does not depend directly on compression crates.
- The CLI and desktop share capability and error models.
- Both desktop UIs can reject invalid creation sources before opening a destination dialog or starting a Worker.
- A backend with security or maintenance issues can be replaced.
- Formats with separate compatibility or licensing questions, including RAR and CAB's limited fixed-layout writer, remain isolated for review.
- Benchmarks, interoperability tests, and fuzzing can run per Provider.
