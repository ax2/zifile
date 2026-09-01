---
title: ADR-0004：隔离归档 Worker
description: 桌面归档操作的进程边界、IPC 与 Windows Job Object 决策。
---

## 状态

已接受，2026-08-24。

## 背景

压缩包解析属于不可信输入处理。仅把任务放在线程池可以避免 UI 卡顿，却不能在解析器崩溃、内存失控或取消失效时保护桌面进程。桌面发行物还必须保持真实进度、密码输入、选择性解压和大型列表能力。

## 决策

- 归档操作通过 `zifile-worker` runtime 执行；桌面便携 EXE 使用 `--zifile-worker` 在自身内启动该 runtime，独立的 `zifile-worker.exe` 继续作为 MSIX 和开发环境的兼容入口。
- `zifile-worker-protocol` 定义显式版本号的 JSON Lines 消息；条目逐条发送，终结事件必须唯一。
- 请求经标准输入发送，密码不进入命令行；请求最多 16 MiB，单事件最多 4 MiB，标准错误最多读取 64 KiB。
- Windows 客户端先创建并配置 Job Object、再发送归档请求。Job 最多一个活动进程、进程内存上限 4 GiB，并启用 kill-on-close。
- 进度事件更新桌面端可观察状态。创建和解压通过版本化控制消息协作取消，包含 7z 单文件读取过程；超过 2 秒仍未退出时才强制回收 Worker。异常退出转为用户可见错误。

## 后果

桌面 UI 不再直接链接调用归档入口，解析器异常通常只终止 Worker。完整可运行目录和 MSIX 保留架构匹配的 Worker 入口；GitHub Release 的便携 EXE 将桌面端与 Worker runtime 合并到单个文件，不要求额外文件。

Job Object 不是权限沙箱：Worker 仍以当前用户身份访问文件。后续可以评估 AppContainer、CPU 时间限制和更细的 Broker，但不能因此宣称当前实现拥有这些能力。
