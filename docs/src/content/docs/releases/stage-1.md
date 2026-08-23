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
- 桌面端加入简体中文/英文切换、系统语言首启检测和主题/语言持久化；设置文件不包含密码。
- 归档表格加入路径搜索和每页 500 项的有界分页，并加入 `Ctrl+O`、`Ctrl+N`、`Ctrl+A`、`Escape` 快捷键。
- 新增版本化 Worker 协议与 `zifile-worker.exe`；桌面列出、校验、解压、创建均跨进程执行。Windows Job Object 限制单进程、4 GiB 内存并启用 kill-on-close；创建和解压先协作取消，2 秒后由进程回收兜底。
- Worker 进度映射到 Windows 任务栏，并在 MSIX 注册 `zifile.exe` App Execution Alias；现代资源管理器右键命令仍需独立 `IExplorerCommand` COM 扩展。

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
- 7z 双向参考互操作已加入同一 Windows 门禁：系统 bsdtar/libarchive 创建的 7z 可由 ZiFile 校验和解压，ZiFile 创建的 7z 可由 bsdtar 解压并核对 Unicode 内容。
- 新增每周定时 fuzz，对路径策略和格式识别各执行 180 秒有界 campaign；失败时保留崩溃产物 14 天。
- 首次手动演练暴露 `cargo-fuzz` 从安装工具环境推断为 musl 目标，ASan 无法与静态 libc 配合；工作流随后与持续集成保持一致，显式固定 `x86_64-unknown-linux-gnu`。修正后的 [演练 32658810750](https://github.com/ax2/zifile/actions/runs/32658810750) 完整通过，两个目标各运行 180 秒，总作业 7 分 50 秒，未产生崩溃产物。
- 桌面回归测试用 100,000 个模拟条目证明单页最多构造 500 行；Windows 实机用 1,200 项 ZIP 验证 3 页翻页、路径搜索和 `Ctrl+N`，并在 1182×810 视口检查中英文与深浅主题无裁切。冷启动复测确认语言/主题从严格的两字段设置文件恢复，测试产生的本地设置随后已清理。
- Worker 协议异常/流式条目单测和真实 IPC 冒烟通过；32 MiB 随机输入的 7z 创建取消测试证明 Worker 在时限内退出，且目标与临时文件均无残留。Windows 实机确认桌面经 Worker 打开并校验 7z。x64 Release MSIX 与完整可运行目录均包含同一次构建的 Worker EXE。
- 隔离 Worker 批次的 [CI 32661216329](https://github.com/ax2/zifile/actions/runs/32661216329) 全部通过，包括格式、Clippy、30 项 Rust 测试、benchmark 编译、真实取消冒烟、第三方互操作、依赖策略、文档和 fuzz 目标编译。[双架构 Release 验证 32661235460](https://github.com/ax2/zifile/actions/runs/32661235460) 同时通过 x64/ARM64 构建、MSIX、产物证明和上传；下载复核确认两个架构均含 Worker、MSIX、Worker 校验和，并生成 Worker 与协议 crate 的 CycloneDX SBOM。该运行未创建正式 GitHub Release。
- 任务栏状态映射新增 2 项单测；本地 x64 Release/MSIX `0.1.0.2` 构建成功，MakeAppx 接受 App Execution Alias manifest。实机打开 32 MiB 7z 验证新包 UI 正常；当前测试桌面自动隐藏任务栏且焦点自动化不稳定，因此未把任务栏视觉效果记为已验证。
- Windows 集成批次的 [CI 32663024457](https://github.com/ax2/zifile/actions/runs/32663024457) 与 [双架构 Release 32663037787](https://github.com/ax2/zifile/actions/runs/32663037787) 全部成功，复验了依赖策略、32 项测试、真实冒烟、x64/ARM64 MSIX、SBOM、产物证明和上传。该运行未创建正式 GitHub Release。
- 建立 opt-in Dioxus/WebView2 可访问 UI 候选，复用现有设置、任务栏和隔离 Worker。候选具备语义首页、归档表格/筛选/分页/选择、完整性校验、解压配置、创建来源/格式/等级/密码、进度/取消及命令行打开。严格全工作区 Clippy 通过；全 feature 运行通过 41 次 Rust 测试（候选二进制的 9 次包含 2 项候选专属测试与 7 项共享模块复验）及两项 benchmark smoke。
- Windows UI Automation 实机识别候选的 landmark、标题、导航、表格、复选框、组合框、滑块、密码输入和 live status。用 ZiFile 创建的 2 文件 ZIP 通过命令行打开，条目列表和 3.8 KB 展开大小正确，随后 Worker 完整性校验成功。1180×760 深色中文首屏、归档页和创建页无裁切。此记录不宣称 Narrator/Accessibility Insights 认证。
- 候选首次云端依赖策略检查只拒绝 `libfuzzer-sys` 的 `NCSA` 和 `target-lexicon` 的 `Apache-2.0 WITH LLVM-exception` 许可证表达式；两者均为 OSI 认可的宽松许可证/例外。策略仅显式增加这两项，没有放宽未知来源、通配依赖或其他许可证检查。

### 遗留问题

- 多任务队列；列出和校验操作目前没有细粒度进度。
- Worker 仍继承当前用户权限，尚未采用 AppContainer；CPU 时间限制和 Broker 模型待后续纵深防御评估。
- 更广泛的第三方 7-Zip/libarchive 语料、直接归档解析器 fuzz、损坏/截断/压缩炸弹扩展语料。
- Shell 命令、任务栏进度、MSIX 安装升级和签名验证。
- 完整键盘遍历、屏幕阅读器、高对比度、中文 IME 和每显示器 DPI 验证；十万条目的峰值内存与交互延迟仍需正式性能基线。
- Iced 当前没有可用于认证的完整 Windows UI Automation/Narrator 语义树；Dioxus 候选已证明 UI Automation 语义树与 Worker 功能路径，但默认替换仍受 Narrator、Accessibility Insights、高对比度、IME、DPI、拖放/快捷键和双架构打包门禁约束。
- Partner Center 名称预留、代码签名、WinGet 与 Microsoft Store 提交。

### 发布结果

进行中。已验证本地 x64 开发 MSIX 和远程 x64/ARM64 非发布产物链；当前提交仍是 Alpha 开发检查点，不是可上架版本，也没有创建公开 Release。演练证据保存在 [GitHub Actions run 32655897142](https://github.com/ax2/zifile/actions/runs/32655897142)。
