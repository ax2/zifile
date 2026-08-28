---
title: 架构总览
description: ZiFile 的分层、crate 边界与任务执行模型。
---

## 分层

```text
Iced UI / Dioxus accessibility candidate ── versioned IPC ── isolated Worker ── zifile-core ── Provider
CLI ────────────────────────────────────────────┘
                                                   │
                                           Windows 文件系统
```

### `zifile-core`

定义格式能力、任务请求、进度、冲突策略、安全限制和统一错误。它不依赖 UI 或 Windows API，桌面端与 CLI 必须共享同一行为。

### 格式 Provider

每个 Provider 明确声明 `list`、`extract`、`create`、`test`、`encryption` 等能力。UI 根据能力显示操作，不通过格式名猜测。

当前使用宽松许可证、可替换的后端组合：`zip`、`sevenz-rust2`、`tar`、`flate2`、`zstd`、`xz2`、`lz4_flex`、`brotli` 和 `bzip2`。具体选择和约束见 ADR-0003。

### 桌面 UI

当前发布基线为 Iced，以单向状态更新组织界面；Dioxus/WebView2 候选验证标准 DOM 到 Windows UI Automation 的语义路径。两者共享同一 Worker IPC，不在 UI 进程解析归档。Windows 特性通过 `windows-rs` 封装。

两套 UI 还共享一个容量为 32（含运行中任务）的内存 FIFO 调度器。打开、重载、校验、解压和创建在提交时快照请求并依次启动独立 Worker；任务完成或取消后才启动下一项。完成消息携带单调 ID，陈旧/重复完成事件不会推进队列。用户可以清空等待项而不误取消当前 Worker；待处理请求随清空或退出直接释放，不持久化其中的路径或密码。

### Worker

桌面端所有列出、校验、解压和创建请求均通过版本化 JSON Lines 协议发送给 `zifile-worker.exe`。归档条目逐条返回，避免十万条目形成单个巨型 IPC 消息。

Windows 客户端在发送请求前把 Worker 放入 Job Object：最多一个活动进程、4 GiB 进程内存上限、Job 关闭时强制结束。创建和解压先通过同一条标准输入通道协作取消；如果 Worker 在 2 秒内没有退出，客户端再强制回收。该边界限制解析器崩溃和内存失控的影响，但 Worker 仍继承当前用户的文件权限，并不等同于 AppContainer 沙箱。详见 ADR-0004。

## 兼容性

目标平台为 Windows 10/11 x64 与 ARM64。Windows 11 可启用现代背景效果，Windows 10 使用不透明降级样式；核心归档 crate 不绑定 Windows，以便测试和未来复用。
