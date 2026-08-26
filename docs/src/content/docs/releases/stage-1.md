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
- 初始评审认为 RAR 的实现和许可需要独立证据，因此当时能力矩阵明确不可用；该历史决策后来由本页 2026-08-25 的纯 Rust RAR Beta 证据更新，不再代表当前能力。
- `bzip2` 的上游使用 bzip2/libbzip2 1.0.6 宽松许可；依赖策略按其 SPDX 标识显式允许，并继续由 cargo-deny 审核。

### 修改

- 建立统一 `create_archive`、`list_archive`、`test_archive` 和 `extract_archive` API。
- 实现 ZIP/ZIP64/AES、7z/AES、TAR、tar.gz、tar.zst、tar.xz、tar.bz2，以及六种单流格式。
- 加入基于签名的检测、安全相对路径、Windows 保留名/ADS 防御、链接拒绝、大小/比例/条目限制、大小写碰撞和四种冲突策略。
- CLI 增加 `list`、`test`、`extract` 和 `create`。
- Iced UI 增加现代首页、归档表格、多选、安全解压、密码、完整性测试、来源管理、拖放、格式、压缩等级、加密创建、深浅主题和后台任务状态。
- 打开、完整性校验、解压与创建均加入条目/字节进度和协作式取消；仅列出归档时也会限制条目数、展开大小和膨胀倍率。
- 冒烟测试从技术壳检查升级为真实 tar.gz 创建、签名检测、列出、校验和解压。
- 建立 x64/ARM64 MSIX、独立 EXE、图标、文件关联、可选签名、SBOM、来源证明和 WinGet 1.12 清单链路。
- 桌面端加入简体中文/英文切换、系统语言首启检测和主题/语言持久化；设置文件不包含密码。
- 归档表格加入路径搜索和每页 500 项的有界分页，并加入 `Ctrl+O`、`Ctrl+N`、`Ctrl+A`、`Escape` 快捷键。
- 两套桌面界面共用分层目录视图：即使归档没有显式目录项也会合成可进入的文件夹，提供根目录面包屑；空搜索仅显示当前层，搜索则跨整个归档保留完整路径，单页仍限制为 500 行。
- 新增版本化 Worker 协议与 `zifile-worker.exe`；桌面列出、校验、解压、创建均跨进程执行。Windows Job Object 限制单进程、4 GiB 内存并启用 kill-on-close；创建和解压先协作取消，2 秒后由进程回收兜底。
- Worker 进度映射到 Windows 任务栏，并在 MSIX 注册 `zifile.exe` App Execution Alias。
- 新增 Rust `cdylib` 实现 Windows 11 `IExplorerCommand`。MSIX 以同一 CLSID 注册 `windows.comServer` 和 `windows.fileExplorerContextMenus`；命令只收集本地选择并用共享 `--create` 启动协议打开创建页，归档工作仍由桌面与隔离 Worker 执行。
- 新增两套桌面 UI 共用的有界 FIFO 操作队列。打开、重载、校验、解压和创建可在 Worker 运行中继续提交，完成/取消后自动启动下一项；状态区显示等待数，并可清空等待项而不取消当前项。队列只驻留内存，调试输出会遮蔽载荷，密码不会进入设置或日志。

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
- 本地 x64 `0.1.0.15` MSIX 已由 MakeAppx 接受现代菜单和 COM surrogate 声明。结构化审计确认 Rust Shell DLL、桌面、CLI、Worker 均为 `0x8664`，CLSID、STA、`*`/`Directory` item type 与 manifest 一致；当前开发包仍不能安装，因此不宣称真实 Explorer 菜单已激活。
- Windows 集成批次的 [CI 32663024457](https://github.com/ax2/zifile/actions/runs/32663024457) 与 [双架构 Release 32663037787](https://github.com/ax2/zifile/actions/runs/32663037787) 全部成功，复验了依赖策略、32 项测试、真实冒烟、x64/ARM64 MSIX、SBOM、产物证明和上传。该运行未创建正式 GitHub Release。
- 建立 opt-in Dioxus/WebView2 可访问 UI 候选，复用现有设置、任务栏和隔离 Worker。候选具备语义首页、归档表格/筛选/分页/选择、完整性校验、解压配置、创建来源/格式/等级/密码、进度/取消及命令行打开。首轮全 feature 运行通过 41 次 Rust 测试；新增快捷键测试后，本地严格 Clippy 与全 feature 复验通过 42 次测试执行（候选二进制 10 项）及两项 benchmark smoke。
- Windows UI Automation 实机识别候选的 landmark、标题、导航、表格、复选框、组合框、滑块、密码输入和 live status。用 ZiFile 创建的 2 文件 ZIP 通过命令行打开，条目列表和 3.8 KB 展开大小正确，随后 Worker 完整性校验成功。1180×760 深色中文首屏、归档页和创建页无裁切。此记录不宣称 Narrator/Accessibility Insights 认证。
- 候选首次云端依赖策略检查只拒绝 `libfuzzer-sys` 的 `NCSA` 和 `target-lexicon` 的 `Apache-2.0 WITH LLVM-exception` 许可证表达式；两者均为 OSI 认可的宽松许可证/例外。策略仅显式增加这两项，没有放宽未知来源、通配依赖或其他许可证检查。
- 修正后的 [CI 32665872620](https://github.com/ax2/zifile/actions/runs/32665872620) 全部通过：依赖策略、格式、Clippy、全 feature 测试、benchmark 编译、真实 Worker 冒烟、第三方互操作、文档和 fuzz 目标编译均成功。
- 候选新增原生拖放处理及 `Ctrl+O`、`Ctrl+N`、`Escape`；实机确认两个 Ctrl 快捷键分别打开原生文件选择器和创建页。CSP 仅允许内联 Dioxus 运行时、本地样式/数据图像、本地 Dioxus 协议和 `127.0.0.1` 回环 WebSocket，非 Dioxus 自定义导航被拒绝；首次过严策略导致空白页，补入 Dioxus 必需的回环 WebSocket后，1182×791 界面与 UI Automation 语义树恢复。
- 核心能力模型新增创建输入形态；ZIP、7z 与 TAR 组合接受文件和目录，gzip、Zstandard、XZ、Bzip2、LZ4 与 Brotli 要求恰好一个文件。Iced 与 Dioxus 均在目标对话框前预检、禁用无效创建并显示双语修复建议。
- 本地 x64 候选 `0.1.0.3` MSIX 与完整可运行目录构建成功；包内候选桌面 EXE 与 Release 构建 SHA-256 一致，可运行目录含桌面、CLI、Worker、MIT 许可证和 README，不含 ZIP。完整目录启动后 UI Automation 再次识别 `main-content` 与 live status。
- [Release 演练 32667737142](https://github.com/ax2/zifile/actions/runs/32667737142) 全部成功：x64、ARM64 和 CycloneDX SBOM 作业通过，两个架构都完成默认与候选 MSIX/EXE 构建、staging、来源证明和上传。下载复核确认每个架构 6 个校验目标全部匹配，x64/ARM64 EXE 的 PE machine 分别为 `0x8664`/`0xAA64`，发布目录无 ZIP。ARM64 物理设备运行仍未验证。
- 候选 `Ctrl+A` 仅在条目区域拥有焦点时拦截，不影响密码/搜索输入；真实窗口从 0 项经单项焦点后全选 2 项，动态 UI Automation 标签从“0 项已选择”更新到“2 项已选择”，live status 同步报告结果。CSS 已加入 `forced-colors: active` 系统颜色映射，但尚未完成 Windows 高对比度实机视觉检查。
- Iced 与候选共用十万条目筛选/分页模块。Criterion 基线为选择性计数 16.90–17.62 ms、有界 500 项页收集 15.46–15.96 ms；五轮优化版桌面基线记录 Iced/候选窗口启动中位数 668.79/294.25 ms，稳定工作集 225.87/405.91 MiB，稳定私有内存 265.05/206.62 MiB。该基线不等同于真实十万项归档端到端峰值。
- 真实 100,000 空条目 ZIP 通过隔离 Worker 在候选版打开，UI Automation 报告 100,000 项、200 页且当前表格仅 500 行；末项搜索和三页筛选导航通过。操作后 7 进程当前工作集/私有内存为 552.70/313.04 MiB，各进程峰值工作集之和 693.72 MiB；后者不是同时刻整树峰值。测试结束应用与 Worker 均退出。
- [CI 32671611160](https://github.com/ax2/zifile/actions/runs/32671611160) 全部成功，复验依赖、格式、严格 Clippy、44 次 Rust 测试、全工作区优化 benchmark 编译、真实 Worker 取消冒烟、ZIP/tar.gz/7z 互操作、文档和 fuzz 目标编译。
- [Release 演练 32671977951](https://github.com/ax2/zifile/actions/runs/32671977951) 全部成功。下载复核确认 x64/ARM64 每架构 6 个校验目标全部匹配，4 个 EXE 的 PE machine 分别统一为 `0x8664`/`0xAA64`，5 个 CycloneDX 1.3 JSON 可解析，发布目录无 ZIP；ARM64 实机运行仍未验证。
- 新增确定性真实十万项浏览器基线脚本。五轮中，首内容中位数/p95 为 3373.80/3668.80 ms，50% 滚动为 195.16/246.34 ms，下一页为 805.87/1143.66 ms；25 ms 同时刻采样的整树最大工作集/私有内存为 669.18/455.71 MiB。该口径包含 UI Automation 观察开销。
- 用 Release 演练的真实 x64/ARM64 SHA-256 重新生成 WinGet 1.12 候选，`winget validate` 成功；尚未提交 winget-pkgs。
- MSIX 开发 Identity 改用微软未签名包固定 OID 并加入错误 Publisher 防护。当前 Windows build 26200 本机仍以 `0x80080204` 拒绝未签名 OID 包，不能记为安装通过；一次不落盘凭据的一日自签名演练已由 SignTool 成功签名，部署仅因测试根不受信任而以 `0x800B0109` 停止。测试证书、私钥和包注册均无残留。
- [Release 演练 32685678567](https://github.com/ax2/zifile/actions/runs/32685678567) 复验新 MSIX 规则并全部通过。下载后 12/12 校验和匹配，8 个 EXE 的 PE machine 与 x64/ARM64 一致，4 个默认/候选 MSIX 均为 `ZiCode.ZiFile.Dev`、固定未签名 OID、版本 `0.1.0.1` 和最低 build 26100；5 个 CycloneDX 1.3 SBOM 可解析，目录无 ZIP。
- 独立键盘回合从干净启动点用 `Tab`/`Enter` 进入归档页和创建页，并用 `Shift+Tab` 返回；主题、语言、`Ctrl+N`、`Ctrl+O` 和文件选择器 `Escape` 均通过。自动化层始终把 WebView 外层报告为焦点，截图绑定也受到测试环境其他窗口干扰，创建表单最终方向键值变更观察被人工终止，因此该证据只计入部分键盘遍历，不宣称可见焦点或完整认证。
- 新增确定性十万条目加载取消基线。最终五轮均由已启用的 UI 取消按钮触发，live status 进入“打开失败: Cancelled”，未出现成功打开状态，确认时对应 Worker 数均为 0；取消完成中位数/p95 为 930.78/1088.73 ms，脚本关闭测试实例并删除临时 ZIP。
- 新增前台句柄保护的创建表单键盘脚本。中文核心流程连续通过两轮；用语言前置切换后的英文整轮也通过。实际焦点顺序为 Home/首页→Archive/压缩文件→Create/创建→主题→语言，反向导航和页面激活通过；Clear/清空、空来源 Create archive/创建压缩文件和空闲 Cancel/取消保持 disabled 且不进入 Tab 顺序；格式用键盘选择 7z，滑块 `6→7→6`，密码可键入并用 Ctrl+A/Backspace 清空且不写入结果。用户切换到飞书时，脚本两次因前台句柄不匹配安全停止并清理实例。

### 遗留问题

- 多任务队列的共享调度器与两套 UI 接线已完成，但真实前台串行/取消/清空冒烟尚待无干扰回合；打开与校验均已接入统一进度与取消。
- Worker 仍继承当前用户权限，尚未采用 AppContainer；CPU 时间限制和 Broker 模型待后续纵深防御评估。
- 损坏、截断、压缩炸弹和更多 libarchive 变体的持续扩展语料。

## 2026-08-26 — 归档打开进度与取消

打开大型归档此前虽在隔离 Worker 中运行且可由桌面端强制回收，但核心 List API 没有统一进度与协作取消。新增兼容旧入口的 `ListOptions` 和 `list_archive_with_options`；ZIP、7z、RAR 1.3–7、CAB、五种 TAR 组合与六种单压缩流均在扫描边界检查取消并推进进度。已知条目总数的格式显示确定进度，TAR/CAB 等总数尚未知时显示双语“正在扫描”，完成后再提交一致的最终总量；单压缩流同时报告实际解码字节。

Worker List 现在复用校验/创建/解压的 100 ms 进度发送器和取消监听器，并保证最终快照先于 `archive_start`/逐项流式结果。两套桌面 UI 无需新协议即可获得实时状态、任务栏进度和现有取消按钮行为。核心往返测试把最终进度不变量扩展到全部 15 类格式，新增预取消回归；真实 Worker 冒烟解析 JSON Lines 并验证最终进度事件顺序。

本地全工作区、全目标、全特性共 90 项 Rust 测试与三个 Criterion 目标通过，严格 Clippy、rustfmt、基础 Worker 冒烟、23 脚本打包策略、nightly 三个 fuzz bin 及 Astro 27 对/55 页零诊断构建通过。未抢占当前用户桌面执行前台 UI 自动化；真实 Narrator 与可见焦点仍属于独立辅助功能门禁。

同一批次把固定损坏头回归从 ZIP、7z、TAR、tar.gz 扩展到全部 15 类格式。每个最小输入都保留足以进入目标 Provider 的签名或扩展提示，List 与完整性校验均必须以普通错误拒绝且不得 panic；持续 fuzz 和真实第三方语料仍作为独立纵深门禁。

CAB 另加入解码阶段损坏回归：保持元数据可正常列出，仅翻转首个 CFDATA 压缩字节；完整性校验和解压均必须失败，原子临时文件不得提交，目标目录保持为空。该测试把固定回归从头部解析推进到真实压缩负载与落盘边界。

## 2026-08-26 — 修改时间保真与浏览

ZIP 创建现在写入源文件和目录的修改时间。ZIP、7z、五种 TAR 组合、RAR 与 CAB 的安全解压都在文件原子提交后恢复可用修改时间；目录时间延迟到所有子项完成，再按路径从深到浅恢复，避免后续写入覆盖父目录元数据。往返测试覆盖两个文件和嵌套目录，固定 RAR 5 与 CAB 夹具则证明只读 Provider 能恢复独立创建的时间字段。

归档列表新增可选结构化时间，包含日历字段、精度，以及 UTC 或时区未指定语义。Worker JSON 在字段缺失时使用默认值、未知时省略，保持 protocol v1 兼容。Iced 与 Dioxus 两套归档表格都显示双语“修改时间”列：Unix/NT 时间明确标记 UTC，传统 ZIP/RAR/CAB 的 DOS 时间显示“无时区”，不虚构偏移。DOS 字段仍只有两秒精度，也不能证明创建者原始时区。

同一个 10 万条目视图模型现在支持按名称、原始大小、压缩大小或修改时间升降序排序。目录始终优先，缺失时间始终在末尾，同值用路径稳定排序；切换排序会回到第一页，实际渲染仍限制为 500 行。Dioxus 表头使用原生按钮与 `aria-sort`，两套 UI 都在当前列显示方向箭头。本机 Windows x64 的 10 万条目名称降序并收集有界页基线为 13.96–15.32 ms。

## 2026-08-24 — 解析器边界加固

### 发现

- 解压入口原先先用全局默认限制列出归档，再在写盘阶段使用调用方的 `ExtractOptions.limits`；调用方收紧的条目数和路径深度没有约束前置解析与列表分配。
- Windows/MSVC 的 `cargo-fuzz` 默认入口参数会破坏 `sevenz-rust2` DLL 链接，关闭该参数又会让最终 fuzz EXE 缺少入口点；该限制不影响 Linux GNU 的正式定时 campaign。

### 修改与验证

- 新增带显式限制的列出和校验 API，解压在创建目标目录前即使用调用方限制。
- 新增 3 项集成测试，覆盖 ZIP/7z/TAR 严格条目上限、目标目录创建前拒绝，以及 ZIP/7z/tar/tgz 损坏头无 panic。
- 新增覆盖全部 13 种已支持格式的 `archive_parsers` libFuzzer 目标，并为输入长度、解析时间、RSS、条目数、展开量、压缩比和路径深度设置边界。
- 本地格式检查、严格 Clippy、59 次全工作区测试和独立 fuzz 工作区 Rust 编译检查通过。Windows 动态链接限制已如实保留，Linux 云端 campaign 结果待远端工作流补证。
- [CI 32733631226](https://github.com/ax2/zifile/actions/runs/32733631226) 全部通过，包括依赖、文档、Linux fuzz 目标编译、固定工具链 Rust 门禁、基准、真实冒烟与互操作。
- 首轮 [动态 fuzz 32733658052](https://github.com/ax2/zifile/actions/runs/32733658052) 的路径策略和格式识别通过；归档目标约执行 569,000 次后发现 292 字节 7z 输入触发 `sevenz-rust2` 文件数量分配 `capacity overflow`。产物已下载并核对 SHA-256 `F193BEF68F1293569F4B5CC256FF829D222E7A2C1CE9DBF85FB7BCC6ABB2CC12`。
- 该输入已转为永久回归测试；7z 列出、校验和解压 Provider 现在把可 unwind 的后端 panic 转成普通错误，OOM 与 sanitizer 失败不截获。修复后全工作区 60 次测试、严格 Clippy 和 nightly fuzz 编译检查通过，云端重跑待补证。
- 归档 fuzz 初始化恢复可 unwind panic hook，使 Provider 边界能在 libFuzzer 进程中按发行语义运行；逃出边界的 panic 仍由外层判失败。首轮 292 字节输入作为文本十六进制固定夹具，每次 campaign 启动都强制重放。
- 同批 [双架构复现 32733631204](https://github.com/ax2/zifile/actions/runs/32733631204) 仍为 x64/ARM64 各 4/5，仅默认 Iced EXE 不同；失败 JSON 已保留，没有将预期失败写成通过。
- 第二轮 [动态 fuzz 32803785688](https://github.com/ax2/zifile/actions/runs/32803785688) 发现另一份 173 字节畸形 7z 可触发 ASan 超大内存分配，产物 SHA-256 为 `FBE1B601781F34CB96699A9114E243B9B8720451B3CC308F4A309EB44BAE90EC`。这证明只捕获 panic 不能构成 OOM 防线。
- 上游 `sevenz-rust2` 0.21.3 起加入损坏归档、无限循环和无界分配加固，当前 0.22.0 包含有界元数据计数；该版本要求 Rust 1.93。项目因此同步升级固定工具链和 CI/Release/复现环境，并把两份崩溃输入都作为共享十六进制夹具，在集成测试和每次解析器 campaign 启动时重放。
- Rust 1.93.0、`sevenz-rust2` 0.22.0 下两份固定样本回归、格式、严格 Clippy、60 次全工作区测试、benchmark 编译、基础与打包策略冒烟、Windows ZIP/tar.gz/7z 互操作、nightly fuzz 编译和 19 页 Astro 构建均通过；单作业 x64 全特性 Release 构建用时 14 分 20 秒并通过。云端依赖策略与文档作业也已通过；完整 CI 与双架构复现仍在运行，不提前记为通过。
- 升级后的 [定向 fuzz 32813469578](https://github.com/ax2/zifile/actions/runs/32813469578) 通过：路径与格式目标按手动参数跳过，归档解析器强制重放两份历史 artifact 后运行 181 秒、执行 498,937 次，最终覆盖计数 4,266、峰值 RSS 370 MiB，未上传新崩溃产物。
- 升级批次 [CI 32813453887](https://github.com/ax2/zifile/actions/runs/32813453887) 四个作业全部通过，复验 Rust 1.93 下的依赖策略、格式、严格 Clippy、60 次测试、benchmark、真实 Worker/打包冒烟、ZIP/tar.gz/7z 互操作、19 页文档和 Linux fuzz 目标构建。
- [双架构复现 32813453959](https://github.com/ax2/zifile/actions/runs/32813453959) 在 Rust 1.93 干净合并提交上仍为 x64/ARM64 各 4/5，只有默认 Iced EXE 不同；两份 JSON 已下载核对。复现脚本随后升级为 schema v2，对不同 PE 记录 headers/section/overlay 哈希与首个差异偏移，并新增无需双构建的诊断器冒烟测试。
- [schema v2 双架构复现 32822543635](https://github.com/ax2/zifile/actions/runs/32822543635) 确认 x64 与 ARM64 的 `.rdata` 首差异都是 `glutin_wgl_sys` 生成绑定内嵌的 `build-a`/`build-b` 隔离 target 路径；headers 差异是 `/Brepro` 内容哈希的后果。双构建现用 `CARGO_ENCODED_RUSTFLAGS` 将每个 target 根重映射到 `Z:\zifile-target`，新云端 5/5 证据产生前不提前勾选路线图。
- [路径重映射复现 32826187552](https://github.com/ax2/zifile/actions/runs/32826187552) 首次在 x64 和 ARM64 同时实现 5/5；两份 JSON 均为 `reproducible=true`，证据哈希分别为 `B7C22C8F3728301BD804AE93AA9DE446645F27C8734955D780CBDAD14EC25C3D` 与 `D6BB828B984B811F56173C9E44418F8C71501A45D86DCF1D4501C0AF7A179DFF`。
- CLI 删除会把密码暴露到进程参数的 `--password <值>`，改为显式 `--password-stdin`；3 项单测与基础冒烟覆盖非空单行读取、空格保留、帮助面策略及 AES 7z 创建/校验/解压。
- 新增官方 7-Zip 双向语料门禁，计划覆盖 7 种参考创建的编码/过滤器/加密组合与 2 种 ZiFile 创建归档，并上传逐文件哈希证据；本机没有 `7z.exe`，因此此项仍等待 GitHub Windows Runner 的首次真实结果，不提前记为通过。
- 首次云端运行 32835391711 在 Deflate 场景发现 `sevenz-rust2` 的对应解码 feature 未启用；既有默认 feature 不包含 Deflate。项目已显式启用后端 `deflate` feature，等待完整语料重跑，失败结果不记为通过。
- 修复后的 [CI 32836336921](https://github.com/ax2/zifile/actions/runs/32836336921) 四个作业全绿；7-Zip 26.02 的 7 种参考创建场景和 2 种 ZiFile 创建场景全部通过完整文件集与 SHA-256 核对，JSON 证据哈希为 `06278BB8B96AB683A3C117BA5E30F1B4AB1CF89F1BBF01E72BAC0CC26B49DB14`。
- 新增可信签名 MSIX 生命周期脚本：显式确认后审计基线/升级包，拒绝覆盖既有安装，验证安装、包内 CLI、升级和 Reset，并在 `finally` 中卸载及输出 JSON。当前没有正式签名包，未执行破坏性生命周期；Reset 与保留数据的 Repair 继续分开记录。
- 用现有结构完整的 x64 开发 MSIX 实测可信签名前置路径：包审计以 `NotSigned` 拒绝，`ZiCode.ZiFile.Dev` 安装数量前后均为 0，未调用安装或改变包注册。
- Shell 命令、任务栏进度、MSIX 安装升级和签名验证。
- 归档页完整表格/解压表单键盘遍历、可见焦点、屏幕阅读器、高对比度、中文 IME 和每显示器 DPI 验证；主导航、创建表单与核心快捷键已有中英文键盘证据。真实十万项归档已覆盖 Worker 列出、首屏有界渲染、搜索、翻页、加载取消及可重复的首内容/滚动/同时刻整树峰值采样。
- Iced 当前没有可用于认证的完整 Windows UI Automation/Narrator 语义树；Dioxus 候选已证明 UI Automation 语义树、Worker 功能路径、核心快捷键、本地 x64 运行及云端 x64/ARM64 打包，但默认替换仍受 Narrator、Accessibility Insights、高对比度、IME、DPI、真实拖放和 ARM64 候选实机运行门禁约束。
- Partner Center 名称预留、代码签名、WinGet 与 Microsoft Store 提交。

### 发布结果

进行中。已验证本地 x64 开发 MSIX、候选运行目录和远程 x64/ARM64 默认/候选非发布产物链；当前提交仍是 Alpha 开发检查点，不是可上架版本，也没有创建公开 Release。最新演练证据保存在 [GitHub Actions run 32685678567](https://github.com/ax2/zifile/actions/runs/32685678567)。

## 2026-08-25 — 纯 Rust RAR 只读 Beta

在 `rars` 0.9.3 提供纯 Rust、禁止 `unsafe`、MIT OR Apache-2.0 且覆盖 RAR 1.3 至 RAR 7 的实现后，项目重新完成了原先暂缓的 RAR 评审。ZiFile 现在报告浏览、校验、解压和加密读取能力，RAR 创建仍明确禁用。

核心测试逐一覆盖 Provider 暴露的九个归档版本，并覆盖固实包选择性解压、Unicode 文件名、加密头、密码错误/缺失、严格资源限制、取消、临时文件提交语义、链接、reparse 属性和 RAR 5+ 重定向。parser fuzz 已加入 RAR，MSIX 增加 `.rar` 文件关联。CI 32853686537 已通过六个有效外部语料和三个不安全链接/重定向拒绝场景；RAR 1.3 与固定上游的已知正确解压树核对，其余五种有效归档与 7-Zip 26.02 逐文件交叉验证。证据 JSON SHA-256 为 `4C52D0240B911609C7DDB0CACB2E484F56C8F886E216347603B228261C4EE8EF`。
