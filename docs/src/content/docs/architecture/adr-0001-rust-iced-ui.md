---
title: ADR-0001：Rust 与 Iced UI
description: 选择 Iced 作为 ZiFile 首选 UI 技术的决定与验证条件。
---

- 状态：暂定，等待 Stage 0 验证
- 日期：2026-08-23

## 决定

产品、核心逻辑和 UI 以 Rust 为主。桌面 UI 首选 Iced，Windows 系统能力通过 `windows-rs` 接入。不使用 Electron；Tauri 不作为首选，因为严格条件下其 UI 主要由 Web 技术实现。

## 理由

Iced 使用 MIT 协议，提供类型安全的单向更新模型、异步任务和 Windows 渲染后端，符合从零编写 Rust UI 的目标。

## 风险

Iced 官方仍将框架描述为实验性软件。Stage 0 必须验证十万行虚拟列表、中文输入法、键盘导航、屏幕阅读器、高对比度、拖放、多显示器 DPI 和 Windows 10/11 渲染。

如果关键验证失败，备选为 `egui`。更换框架不能影响 `zifile-core` 或格式 Provider。
