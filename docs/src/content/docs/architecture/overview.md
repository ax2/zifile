---
title: 架构总览
description: ZiFile 的分层、crate 边界与任务执行模型。
---

## 分层

```text
Iced UI / CLI
      │
应用状态与任务队列
      │
zifile-core
      │
格式 Provider
      │
隔离 Worker
      │
Windows 文件系统
```

### `zifile-core`

定义格式能力、任务请求、进度、冲突策略、安全限制和统一错误。它不依赖 UI 或 Windows API，桌面端与 CLI 必须共享同一行为。

### 格式 Provider

每个 Provider 明确声明 `list`、`extract`、`create`、`test`、`encryption` 等能力。UI 根据能力显示操作，不通过格式名猜测。

当前使用宽松许可证、可替换的后端组合：`zip`、`sevenz-rust2`、`tar`、`flate2`、`zstd`、`xz2`、`lz4_flex`、`brotli` 和 `bzip2`。具体选择和约束见 ADR-0003。

### 桌面 UI

首选 Iced，以单向状态更新组织界面；Windows 特性通过 `windows-rs` 封装在独立 crate 中。耗时工作只返回消息和进度，不在 UI 线程执行。

### Worker

归档解析最终放入独立进程。Stage 2 先用 Job Object 限制内存、CPU 和子进程，后续评估 AppContainer。

## 兼容性

目标平台为 Windows 10/11 x64 与 ARM64。Windows 11 可启用现代背景效果，Windows 10 使用不透明降级样式；核心归档 crate 不绑定 Windows，以便测试和未来复用。
