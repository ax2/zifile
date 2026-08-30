---
title: Stage 4 工作日志
description: ZiFile 1.0 稳定发布的准备、合同冻结与跨渠道门禁记录。
---

## 目标

冻结公开 CLI/Provider 合同，完成用户与发布文档，获得签名和认证证据，并从同一版本源发布 GitHub、WinGet 与 Microsoft Store 1.0。

## 当前状态

Stage 4 尚未完成。公开契约候选、1.0 readiness manifest、发布 workflow、SBOM、来源证明、校验和和阶段档案已存在，但这只能证明发布链路已准备，不能证明稳定版本已满足所有门禁。

## 已准备

- `docs/src/content/docs/development/contracts.md` 定义 CLI 命令、格式值、冲突策略、密码输入、退出码、核心 Provider 和 Worker 协议边界。
- `tests/smoke/contract-policy.ps1` 已将候选 CLI 命令、15 个创建格式、17 行能力矩阵、双语契约页和退出码接入 Windows CI；最终冻结仍保留到 1.0 发布提交。
- `release/readiness.json` 将稳定标签绑定到辅助功能、队列、可信安装、ARM64、截图、WACK、WinGet、Store、Partner Center 和签名证据。
- Release workflow 从 workspace 版本生成双架构构建、审计、校验和、SBOM、来源证明和 GitHub Release；普通公开发布使用未签名产物，正式签名可通过 workflow 输入显式开启。
- `v0.1.0-alpha.1` 已作为公开 prerelease 发布；`v0.1.0`、`v0.1.2`、`v0.1.3`、`v0.1.4` 和 `v0.1.5` 均通过匹配 tag 自动生成公开 GitHub Release。

## 必须完成

- 关闭真实前台队列、可信签名安装生命周期、物理 ARM64、完整辅助功能和 WACK 门禁。
- 完成 Partner Center 身份、SignPath/生产签名以及正式双语 Store 截图、WinGet 接受和 Store 认证。
- 在 1.0 发布提交上冻结 CLI、Provider 和 IPC 合同，并更新最终版本说明、Stage 4 证据和发布结果。

## 发布结果

稳定 1.0 尚未发布；`v0.1.5` 是当前面向 GitHub 的公开可用版本，但正式 Store、WinGet 和可信签名门禁仍未完成。Stage 4 保持进行中。

## 2026-08-29 正式公开版本结果

- [`v0.1.0` Release](https://github.com/ax2/zifile/releases/tag/v0.1.0) 已发布；tag 与 `main` 提交 `a601738` 一致，Release 为非 Draft、非 prerelease。
- 发布工作流生成并上传 x64/ARM64 MSIX、桌面端/CLI/Worker/Shell 可运行文件、SHA-256、SBOM、审计文件和 WinGet 清单候选；Release 明确标注为未签名 GitHub 构建。
- 这次发布验证了 GitHub 分发链路，但不关闭可信签名、WinGet 社区接纳、Microsoft Store、WACK、物理 ARM64 和完整辅助功能门禁。

## 2026-08-29 质量收口记录

- 两套桌面归档页新增双语“解压全部”主操作，同时保留仅解压当前选择的次操作；从资源管理器打开归档时仍自动解压全部内容，避免用户必须先手动全选。
- 两套桌面归档表格新增双语“复制校验和”操作：Iced 使用系统剪贴板，可访问 Dioxus UI 使用 Clipboard API，并在能力缺失或 Promise 失败时进入错误状态；对应文案、接线和失败回退已有测试覆盖。
- 两套桌面归档页标题栏新增“在资源管理器中显示”操作，调用 Windows Explorer 选中当前归档；启动失败进入错误状态，路径不会写入设置或日志。
- 可访问归档浏览器在大归档的高频进度刷新期间改用轻量摘要，保留取消、排队、打开其他文件和资源管理器定位按钮，任务完成后恢复表格，避免反复扫描 100,000 条目阻塞前台队列操作。
- standalone `.lzma` 已从原先复用 XZ 枚举的只读别名拆为独立 `ArchiveFormat::Lzma`：核心使用 `lzma-rust2` 读写 LZMA-alone，CLI、Iced、Dioxus、能力矩阵和 Windows Foundation smoke 均提供独立的 `lzma` 选项；`.xz` 的显示和检测不再与 LZMA 混淆。
- Explorer 创建命令现在在 Shell 菜单层拒绝已消失的路径、非文件系统虚拟项目和符号链接，并保留命令行长度与路径去重保护；Shell 回归测试增至 19 项，覆盖真实文件/文件夹正向路径和失效来源拒绝。
- 解压路径现在在核心层拒绝目标本身及已有输出父路径中的符号链接、junction 和 reparse point，并用回归测试确认不会向链接目标写入；归档内链接拒绝与宿主目标路径检查保持一致。

- 文档 locale 检查现在同时要求 31 对中英文页面存在且正文非空；剥离 front matter 后的空 Markdown 页面会使检查失败，防止文件存在但生成页面为空白。
- 完整 `cargo test --workspace --all-targets --all-features --locked` 通过，覆盖 CLI、core、42 项归档回归、Iced、可访问候选、Shell、Worker、协议和 Criterion 基准目标。
- 默认 Iced 窗口现在居中启动，并限制为不小于 920×620，避免归档表格、搜索工具栏和创建控件在调整窗口时被压缩到不可用；新增窗口设置回归测试。
- Astro 静态构建生成 63 个页面，0 errors/warnings/hints；用户文档、Node/PowerShell 语法、打包策略和工作树卫生检查通过。
- 这些是本地代码与文档证据，不改变 11 项外部发布门禁的 `pending` 状态，也不替代可信签名、Store/WinGet、ARM64 实机、WACK 或真实辅助技术验证。

## 2026-08-30 — 0.1.2 发布准备

- 将 workspace、内部 crate 依赖、文档包和 Cargo.lock 版本统一推进到 `0.1.2`，为 all-in-one MSIX 与便携 EXE 发布资产创建稳定补丁版本。
- 版本仍通过普通公开 Release 流程生成未签名的 GitHub Windows 产物；SignPath、WinGet 社区接纳、Microsoft Store、WACK、物理 ARM64 和完整辅助技术门禁继续保持独立状态。

## 2026-08-30 — 0.1.2 正式发布结果

- [`v0.1.2 Release`](https://github.com/ax2/zifile/releases/tag/v0.1.2) 已由 tag 自动发布；Release 为非 Draft、非 prerelease，发布工作流 [33291234378](https://github.com/ax2/zifile/actions/runs/33291234378) 成功完成。
- 公开资产严格收敛为一个 all-in-one `msixbundle`、一个 x64 独立便携 EXE、一个 ARM64 独立便携 EXE，以及 `SHA256SUMS.txt`；DLL、构建配置、SBOM 和 WinGet 清单候选只保留在工作流证据中。
- 独立 EXE 将自身以内部 Worker 模式重新启动，不再要求用户额外下载或摆放 Worker；Release 仍明确标记为未签名 GitHub 构建。

## 2026-08-30 — 0.1.3 独立便携版本发布

- [`v0.1.3 Release`](https://github.com/ax2/zifile/releases/tag/v0.1.3) 已由工作流 [33296532873](https://github.com/ax2/zifile/actions/runs/33296532873) 自动发布；PR #39 的 9 项 CI 均通过，其中包括 x64 和 ARM64 可复现双构建。
- 最终公开资产严格为 `ZiFile-0.1.3.0-windows.msixbundle`、`zifile-windows-x64.exe`、`zifile-windows-arm64.exe` 和 `SHA256SUMS.txt`。桌面 EXE 内置 Worker runtime，并使用 `--zifile-worker` 参数启动自身，因此便携版不需要额外的 Worker 文件。
- x64 便携 EXE 已从 Release 下载并与 `SHA256SUMS.txt` 匹配；ARM64 哈希保留在工作流生成的校验清单中。该 Release 仍明确标记为未签名，也不是 Microsoft Store 认证包。

## 2026-08-30 — 0.1.4 正式公开版本结果

- [`v0.1.4 Release`](https://github.com/ax2/zifile/releases/tag/v0.1.4) 已由工作流 [33315987748](https://github.com/ax2/zifile/actions/runs/33315987748) 自动发布；PR #51 的版本一致性、Rust 质量、互操作性、性能、Fuzz 和 x64/ARM64 可复现双构建检查全部通过。
- 最终公开资产严格为 `ZiFile-0.1.4.0-windows.msixbundle`、`zifile-windows-x64.exe`、`zifile-windows-arm64.exe` 和 `SHA256SUMS.txt`；没有单独发布 DLL、JSON/YAML 配置、SBOM 或 provenance 文件。
- x64 和 ARM64 Windows 发布 job、all-in-one MSIX bundle job、SBOM job 和 Release 发布 job 均成功；签名 job 因 SignPath 尚未配置而按条件跳过，Release 仍明确为未签名 GitHub 构建。

## 2026-08-30 — 公开 Release 资产审计修复

- 发现阶段预发布 Job 会直接调用仓库内的 `tests/smoke/public-release-assets.ps1`，但 Job 未 checkout 当前提交；在干净的 Ubuntu runner 上会因脚本不存在而失败。
- PR [#45](https://github.com/ax2/zifile/pull/45) 在 `publish-stage` 的第一步加入仓库 checkout，并增加作用域检查，确保该 Job 在执行公开资产审计前始终拥有当前版本的脚本。
- 合并提交为 [`4d65881`](https://github.com/ax2/zifile/commit/4d65881c5c20d3a6cb8221d6d98aa94c90d7775a)；合并后的主分支 CI [33305272788](https://github.com/ax2/zifile/actions/runs/33305272788) 全部通过。
- 该修复不改变公开资产集合：Release 仍只发布 all-in-one MSIX、x64 独立 EXE、ARM64 独立 EXE 和 `SHA256SUMS.txt`；审计、SBOM、来源证明和 WinGet YAML 继续作为 workflow artifact 保留。

## 2026-08-30 — UI 校验与 WinGet 门禁稳定性

- PR [#48](https://github.com/ax2/zifile/pull/48) 在 Iced 创建页面直接显示非空来源列表的校验失败提示，并保持“创建”按钮禁用；新增回归守卫确认双语危险提示会进入视图。
- PR 在合并提交 [`1487bf6`](https://github.com/ax2/zifile/commit/1487bf6758d8a69b4844a0a4709e146a13ee0c0a) 前通过 `cargo fmt --all -- --check` 和 `cargo test -p zifile-desktop --all-targets --locked`。
- PR [#49](https://github.com/ax2/zifile/pull/49) 为固定版本的 WinGet validation client 增加最多 3 次有界退避重试，用于处理 CDN 瞬时连接失败，并将尝试次数写入验证证据。
- PR #49 的全部检查通过，包括官方 WinGet manifest validation、Rust quality、Foundation smoke、性能、模糊目标编译和参考工具互操作性；合并提交 [`cbb6505`](https://github.com/ax2/zifile/commit/cbb6505be639f27b064b98de52c766ebea0ec14d) 已进入 `main`。

## 2026-08-30 — WinGet all-in-one 清单收口

- WinGet 生成器、验证器和 Release workflow 已删除对未公开 x64/ARM64 单架构 MSIX URL、哈希和本地路径的依赖，只接收公开 all-in-one `.msixbundle`。
- 安装器清单继续提供 x64 与 ARM64 两个选择节点，但验证器要求它们引用同一个 bundle URL 和同一 SHA-256，并核对本地 bundle；公开下载面与 WinGet 下载面由此使用同一安装包。
- 本地 WinGet 1.29.290 的官方 `validate` 接受 schema 1.12 四文件候选；29 个扩展名同步、哈希篡改拒绝和完整 packaging policy 均通过。社区仓库接受与签名门禁仍未完成。

## 2026-08-31 — 默认桌面快捷键可发现性

- 默认 Iced 桌面端原有 `Ctrl+O` 打开、`Ctrl+N` 创建、归档页 `Ctrl+A` 全选、`F1` 帮助和 `Esc` 取消，但此前没有面向用户的可见说明。
- 默认 Iced 与可访问候选的“关于”页现均以双语键帽列表展示五项快捷键及其作用；默认 UI 的源代码回归测试将显示组合与实际键盘映射绑定，避免后续只修改其中一侧。
- `cargo fmt --all -- --check`、`cargo test -p zifile-desktop --all-targets --all-features --locked` 与全 workspace Clippy 通过，覆盖桌面共享库 32 项、默认应用 37 项、可访问候选 38 项测试和 6 项 10 万条目浏览基准。该代码级证据不替代真实前台键盘遍历、Narrator、高对比度和焦点可见性验收。

## 2026-08-31 — 创建密码生命周期收口

- 默认 Iced 与可访问候选此前会在创建请求已经进入执行或内存队列后继续把密码留在表单状态；这与公开隐私说明中的临时保留边界不够一致。
- 两套 UI 现在只在请求被接受时立即清空创建表单密码；队列已满、请求未被接受时保留输入供重试。Worker 或队列中的请求快照仍按完成、清空或退出释放，不写入设置和日志。
- 新增双实现单测覆盖“接受即清空、拒绝保留、非创建请求不清空创建字段”。归档解密密码继续限定在当前归档会话，以支持后续校验和解压。

## 2026-08-31 — 完成后输出定位

- 两套桌面 UI 的状态栏新增双语“查看输出”动作：创建成功时在资源管理器中选中生成的压缩文件，解压成功时定位输出目录。
- 输出路径只来自 `Create`/`Extract` 请求快照且只在结果类型正确、操作成功时公开；开始下一任务会清除旧路径，失败、取消和 Worker 协议不匹配不会显示陈旧动作。
- 默认 Iced 与可访问候选分别新增请求路径和界面接线回归，桌面共享库 32 项、默认应用 39 项、可访问候选 40 项测试及 6 项 10 万条目基准通过。

## 2026-08-31 — 主分支合并门禁

- PR #55 暴露出 `main` 未受保护：请求自动合并时 GitHub 立即合并，没有等待正在运行的 CI；代码此前已有完整本地验证，事后远端检查继续运行，但流程本身不满足上架级治理。
- `main` 现要求 PR、最新分支上的七项 CI、已解决对话和线性历史；管理员同样受约束，强推与分支删除被禁止。为适配单维护者项目，批准数保持 0，不制造必须由第二个账号审批的死锁。
- GitHub API 回读确认 `strict=true`、`enforce_admins=true`、七个检查上下文完整、`required_approving_review_count=0`、线性历史与对话解决启用、强推和删除关闭。PR #56 将作为启用后的首次合并验证。

## 2026-08-31 — 0.1.5 发布准备

- 自 0.1.4 后累计的创建来源校验、all-in-one WinGet 清单、快捷键帮助、创建密码生命周期和完成后输出定位已形成一个可用补丁发布节点。
- workspace、三个内部依赖约束、六个 workspace lock 条目和 Astro 文档包统一升级到 `0.1.5`；版本门禁确认 tag `v0.1.5` 与 MSIX `0.1.5.0` 映射一致。
- `CHANGELOG.md` 已整理为顶部空 `[Unreleased]`、随后 `0.1.5` 和旧版本的标准逆序结构；0.1.5 包含 6 条发布项并通过 tag-ready 校验。下载文档在 Release 真正成功前继续指向 v0.1.4。

## 2026-08-31 — 0.1.5 正式发布结果

- PR [#57](https://github.com/ax2/zifile/pull/57) 在 7 项常规质量门禁和 x64/ARM64 可复现双构建全部通过后合并；注释 tag `v0.1.5` 精确指向 merge commit `8959f3d0042bf9ba29eed299a416bf952821b0c1`。
- [Release workflow 33327132468](https://github.com/ax2/zifile/actions/runs/33327132468) 成功完成工作区测试、双架构构建、x64 独立 EXE 冒烟、ARM64 PE 架构审计、all-in-one MSIX Bundle、来源证明和公开发布。
- [v0.1.5 Release](https://github.com/ax2/zifile/releases/tag/v0.1.5) 为非草稿、非预发布版本，公开资产严格为 `ZiFile-0.1.5.0-windows.msixbundle`、`zifile-windows-x64.exe`、`zifile-windows-arm64.exe` 和 `SHA256SUMS.txt`，没有附带 DLL、JSON、YAML、SBOM 或来源证明内部文件。
- 发布后重新下载四个公开文件；MSIX Bundle、x64 EXE 和 ARM64 EXE 的 SHA-256 均与 `SHA256SUMS.txt` 及 GitHub asset digest 一致。该证据证明 GitHub 发布完整性，不替代可信签名、WACK、Store、WinGet 或物理 ARM64 运行门禁。

## 2026-08-31 — 归档重新加载快捷键

- 两套桌面 UI 原有“重新加载”按钮，但键盘用户需要遍历到按钮才能刷新已打开归档；文件管理器常用的 `Ctrl+R` 未接入，也未出现在快捷键帮助中。
- 默认 Iced 与可访问候选现在把精确 `Ctrl+R` 映射到现有重新加载队列路径；没有当前或待打开归档时保持无操作，不猜测路径。可访问候选的重新加载/解锁按钮公开 `aria-keyshortcuts="Control+R"`。
- 中英文关于页和用户文档同步加入快捷键；键盘映射、精确修饰键、可见帮助和辅助技术元数据由两套 UI 单测锁定。真实前台键盘与 Narrator 复验仍保留为外部门禁。

## 2026-08-31 — 归档搜索快捷键

- 两套桌面 UI 新增精确 `Ctrl+F`：已有归档时切回归档页并聚焦搜索框，同时保留当前查询；没有归档时不拦截快捷键。
- 可访问候选的搜索框公开 `aria-keyshortcuts="Control+F"` 和稳定控件标识；中英文关于页与桌面文档同步说明该操作。
- 单测覆盖精确修饰键、无归档边界、可见帮助和语义元数据。真实前台焦点移动与屏幕阅读器复验仍属于外部门禁。

## 2026-08-31 — 关闭归档会话

- 两套桌面 UI 的归档标题栏新增“关闭压缩文件”，并支持精确 `Ctrl+W`；关闭后返回首页，不退出应用。
- 关闭会释放当前归档密码、归档元数据、选择、搜索、分页和文件夹导航状态。正在运行或队列交接中的工作会禁用该动作，避免完成事件重新写回已经关闭的会话。
- 中英文关于页和桌面文档同步说明该操作；单测覆盖精确修饰键、忙碌边界、会话清理、可见帮助和 `aria-keyshortcuts="Control+W"`。真实前台键盘与辅助技术复验仍属于外部门禁。
