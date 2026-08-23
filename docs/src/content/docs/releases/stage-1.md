---
title: Stage 1 工作日志
description: ZiFile Alpha 阶段的真实归档核心、桌面流程和验证记录。
---

## 目标

交付可以真实创建、浏览、校验和安全解压主要格式的 Alpha 主线，并让 CLI 与桌面端共享完全相同的核心行为。

## 2026-08-24

### 发现

- Stage 0 只有格式枚举、扩展名识别和 UI 占位，不能证明任何归档格式可用。
- `zip` 8.x、`sevenz-rust2` 0.20、`tar`、`flate2`、`zstd`、`xz2`、`bzip2`、`lz4_flex` 和 `brotli` 与当前 Rust 1.88 工具链及 MIT 项目许可边界兼容。
- RAR 的实现和许可需要独立评审；本阶段将能力矩阵改为明确不可用，避免虚假声明。
- `bzip2` 的上游使用 bzip2/libbzip2 1.0.6 宽松许可；依赖策略按其 SPDX 标识显式允许，并继续由 cargo-deny 审核。

### 修改

- 建立统一 `create_archive`、`list_archive`、`test_archive` 和 `extract_archive` API。
- 实现 ZIP/ZIP64/AES、7z/AES、TAR、tar.gz、tar.zst、tar.xz、tar.bz2，以及六种单流格式。
- 加入基于签名的检测、安全相对路径、Windows 保留名/ADS 防御、链接拒绝、大小/比例/条目限制、大小写碰撞和四种冲突策略。
- CLI 增加 `list`、`test`、`extract` 和 `create`。
- Iced UI 增加现代首页、归档表格、多选、安全解压、密码、完整性测试、来源管理、拖放、格式、压缩等级、加密创建、深浅主题和后台任务状态。
- 解压与创建加入真实字节/条目进度和协作式取消；仅列出归档时也会限制条目数、展开大小和膨胀倍率。
- 冒烟测试从技术壳检查升级为真实 tar.gz 创建、签名检测、列出、校验和解压。
- 建立 x64/ARM64 MSIX、独立 EXE、图标、文件关联、可选签名、SBOM、来源证明和 WinGet 1.12 清单链路。

### 验证

- 9 个核心单元测试通过，包括路径穿越、Windows 保留名和深度限制。
- 12 个集成测试通过，覆盖全部已声明格式的往返、加密 ZIP、恶意 ZIP、TAR 链接、选择性解压、取消、膨胀限制和冲突策略。
- 在 Windows 真实启动桌面程序，检查 1180×780 深色首页、创建页、空归档页和原生文件选择器；未发现裁切或阻塞 UI 线程的问题。
- 8 MiB 本地基准中，ZIP 创建约 262–275 MiB/s，完整性校验约 3.04–3.15 GiB/s；该数字只作为首轮机器基线。
- 在本机 Windows SDK 上成功生成 `ZiFile-0.1.0.0-windows-x64.msix` 和完整可运行目录；当前为开发 Identity、未签名包，不构成 Store 认证证据。
- 本机没有 Visual C++ ARM64 交叉工具链，`zstd-sys` 本地交叉编译因此被正确阻止；Windows 2022 Runner 已成功完成全部 ARM64 Rust/C 依赖和应用的优化构建。首次打包随后发现脚本误运行 ARM64 版 `MakeAppx.exe`；现按宿主架构选择 SDK 工具，同时仍把包清单标记为目标架构。
- 首次非发布打包演练确认 `cargo-cyclonedx` 0.5.9 按 crate 输出 `*.cdx.json`；收集规则已改为匹配真实产物名，而不是旧版文档中的 `bom.json`。
- GitHub Actions 非发布演练 `32655897142` 全部通过：Windows x64、Windows ARM64 和 CycloneDX SBOM 三个作业成功，公开 Release 作业按设计跳过。两个架构均完成优化构建、MSIX、独立 EXE、SHA-256、来源证明和 artifact 上传。
- Windows PowerShell ZIP 与系统 bsdtar 的双向互操作通过：两类参考工具创建的 ZIP/tar.gz 可由 ZiFile 校验和解压，ZiFile 创建的包也可由参考工具解压并核对 Unicode 文件内容。

### 遗留问题

- 多任务队列和独立 Worker 进程；TAR/7z 后端的单个超大文件目前只能在条目边界更新部分状态。
- 7z 与更广泛 libarchive 互操作语料、定时 fuzz、损坏/截断/压缩炸弹扩展语料。
- Shell 命令、任务栏进度、MSIX 安装升级和签名验证。
- 键盘、屏幕阅读器、高对比度、中文 IME、DPI 和十万条目验证。
- Partner Center 名称预留、代码签名、WinGet 与 Microsoft Store 提交。

### 发布结果

进行中。已验证本地 x64 开发 MSIX 和远程 x64/ARM64 非发布产物链；当前提交仍是 Alpha 开发检查点，不是可上架版本，也没有创建公开 Release。演练证据保存在 [GitHub Actions run 32655897142](https://github.com/ax2/zifile/actions/runs/32655897142)。
