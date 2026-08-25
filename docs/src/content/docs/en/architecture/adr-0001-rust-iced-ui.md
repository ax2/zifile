---
title: "ADR-0001: Rust and Iced UI"
description: The decision to use Iced as ZiFile's initial UI and its validation conditions.
---

- Status: provisional, pending validation
- Date: 2026-08-23

## Decision

The product, core logic, and UI are primarily Rust. Iced is the initial desktop UI, with Windows capabilities provided through `windows-rs`. Electron is excluded. Tauri is not the first choice because its UI is primarily web technology under the project's strict Rust-first requirement.

## Rationale and risk

Iced is MIT-licensed and offers a type-safe unidirectional update model, asynchronous tasks, and a Windows renderer. Iced also describes itself as experimental. Validation therefore covers a bounded 100,000-row list, Chinese IME, keyboard navigation, screen readers, high contrast, drag-and-drop, multi-monitor DPI, and Windows 10/11 rendering.

If critical validation fails, the UI may be replaced without changing `zifile-core` or format Providers. The current accessibility candidate and migration decision are documented in ADR-0005.
