---
title: Stage 2 工作日志
description: ZiFile Beta 阶段的 Windows 集成、隔离 Worker 与分发产物记录。
---

## 目标

完成 Windows 日常工作流：文件关联、拖放、任务栏反馈、隔离 Worker、双架构包，以及 Explorer 集成的可验证实现。

## 证据范围

本页根据当前代码、打包审计、已保存的 CI/Release 结果和当前工作树整理。缺少正式签名安装、物理 ARM64 和真实 Explorer 生命周期的历史证据，不将这些内容写成已完成。

## 已交付

- 桌面端通过版本化 JSON Lines 协议启动 `zifile-worker.exe`，并使用 Windows Job Object 约束 Worker 生命周期、内存和关闭回收。
- ZIP/7z/TAR 家族和单流格式的文件关联、App Execution Alias、任务栏进度、桌面拖放以及可运行目录均接入统一核心能力模型。
- MSIX 和独立 EXE 支持 x64 与 ARM64；构建流程生成校验和、SBOM、来源证明和包审计。
- Windows 11 Explorer 集成由纯 Rust `IExplorerCommand` DLL 提供创建和解压命令。创建命令覆盖选中文件、文件夹和 `Directory\Background`；解压命令只对单个受支持归档显示。
- Shell DLL 只收集文件系统路径并启动可见桌面，实际解析、密码、进度、取消和安全限制仍在桌面与隔离 Worker 中完成。

## 验证

- 已记录的 Windows 集成 CI `32663024457` 和双架构 Release 演练 `32663037787` 成功完成依赖、Rust 测试、真实 Worker 冒烟、MSIX、SBOM、来源证明和产物上传。
- Release 演练 `33184684164` 成功生成 x64/ARM64 产物，并在 Alpha 预发布路径跳过正式签名、Store 和 WinGet 门禁。
- 当前本地 x64 MSIX 审计确认桌面、CLI、Worker 和 Shell DLL 均为 `0x8664`，Shell 清单包含 `*`、`Directory` 和 `Directory\Background` 三种创建上下文；该包为 `NotSigned`。

## 遗留问题

- [#12](https://github.com/ax2/zifile/issues/12)：可信签名包的安装、升级、Repair/Reset、卸载和 Explorer 激活/清理仍未完成。
- [#13](https://github.com/ax2/zifile/issues/13)：物理 ARM64 Windows 运行证据仍缺失。
- 未签名开发包不能作为正式 Store 或真实 Explorer 生命周期证据；不能用清单存在替代安装验证。

## 发布结果

Beta 交付的代码与非发布产物链已实现并可审计；正式 Beta 仍未宣称完成，等待可信签名和实机生命周期证据。

## 2026-08-29 — Shell 能力收敛与创建预检

### 修改

- Shell 解压命令改为在允许慢查询时复用核心 `detect_format` 与格式能力，不再维护独立后缀白名单；有效归档改名后仍可发现，无效文件不会仅凭 `.zip` 后缀显示，且 Explorer 项目必须是实际文件。
- 两套桌面 UI 的创建预检新增“来源已不存在”状态，在打开保存对话框前给出双语恢复提示；Worker 仍在执行时重新校验来源。

### 验证

- `cargo test --workspace --all-features --locked` 通过；Shell 14 项测试包含 archive-named directory 回归，严格 Clippy 通过。
- `Build-Package.ps1 -Version 0.1.0.1 -Architecture x64` 成功生成可运行目录与开发 MSIX；包审计确认四个 PE 均为 x64、Shell 上下文包含 `*`、`Directory`、`Directory\Background`，包保持 `NotSigned`。MSIX SHA-256 为 `A9491A363ABFA878D53BF72F964504F89D6E422D272CAA6E0DD2ED6DFEBBD000`。
