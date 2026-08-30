---
title: 构建与发布
description: GitHub、WinGet 与 Microsoft Store 的统一版本发布流程。
---

## 发布产物

- Windows x64 与 ARM64 桌面程序和 CLI。
- MSIX 安装包与无需安装的独立 EXE。
- SHA-256 校验文件。
- 每个 MSIX 的结构化包审计 JSON。
- CycloneDX JSON SBOM。
- GitHub 构建来源证明。
- 版本更新日志和对应文档快照。

## 渠道

GitHub Release 面向用户只保留一个 all-in-one MSIX、一个 x64 独立便携 EXE、一个 ARM64 独立便携 EXE，以及一个 SHA-256 校验文件。审计 JSON、SBOM、来源证明和 WinGet YAML 作为 workflow artifact 保留，不混入 Release assets。GitHub Release 是公开构建的第一落点。WinGet manifest 使用计划 ID `ZiCode.ZiFile` 并引用发行方控制的版本化下载地址。Microsoft Store 以 MSIX 为主。

WinGet 候选只接收公开 all-in-one `.msixbundle` 的 URL、SHA-256 和本地验证路径，不再要求或引用未公开的单架构 MSIX。清单按微软 MSIXBundle 示例保留 x64 与 ARM64 两个安装器节点，两者必须指向同一个 bundle URL 和同一哈希；`Test-Manifests.ps1` 会同时锁定该关系、29 个文件扩展名和本地 bundle 哈希，再交给官方 `winget validate`。

Release Job 会运行 `tests/smoke/portable-exe.ps1`：x64 桌面 EXE 被复制到不含独立 Worker 的目录中，启动内置的 `--zifile-worker` 模式，读取真实 ZIP 并校验返回项目。ARM64 EXE 在 x64 runner 上执行相同的 PE 头与“无旁置 Worker”审计，但跳过运行；真正的 ARM64 执行仍属于实机门禁。

文档当前发布到 `ax2.github.io/zifile`；完成 DNS 配置后迁移到计划域名 `zifile.zicode.com`。

Partner Center 需要先手动预留名称并完成首个提交；之后可以接入 Store Submission API。签名策略见 [ADR-0006](/zifile/architecture/adr-0006-release-signing/)：Store 完成商店通道最终签名，GitHub/WinGet 使用云 HSM 托管的公开受信任签名，生产私钥不得导出到 GitHub Secret。Release workflow 已移除 PFX，并接入 DigiCert Binary Signing 的受保护 simple-signing 路径；没有真实证书证据前，1.0 签名门禁仍保持 `pending`。

公开的[代码签名政策](/zifile/development/code-signing-policy/)记录 SignPath Foundation 申请状态、发布角色、来源证明边界、隐私说明以及独立的 Partner Center MSIX 身份。申请获批且 MSIX 身份决策完成评审前，不将 SignPath Foundation 接入正式发布工作流。

推送 `v*` 标签会为 x64 和 ARM64 构建 MSIX 与独立 EXE，并直接发布公开的 GitHub Release。独立 EXE 将自身以内部 Worker 模式重新启动，因此不需要旁边的 Worker 文件；稳定标签（例如 `v1.0.0`）默认只发布用户需要的一个 all-in-one MSIX、一个 x64 独立便携 EXE、一个 ARM64 独立便携 EXE 和一个 SHA-256 校验文件。审计 JSON、SBOM、WinGet 清单候选和构建证明由 workflow 作为内部证据保留，不混入 Release assets。Release Notes 会明确提示未签名状态，用户应先核对 SHA-256。带连字符的阶段标签（例如 `v0.1.0-alpha.1`、`v0.1.0-beta.1`、`v1.0.0-rc.1`）发布相同的用户资产到 GitHub Pre-release；它们不能提交 WinGet 或 Store。正式签名仍保留为 Release workflow 的 `digicert-stm` 手动选项，只有显式勾选 `require_release_ready` 时才要求 Partner Center、Store 截图和全部 1.0 readiness 门禁。未签名 `.Dev` 包使用微软固定 OID Publisher 并要求 Windows 11 build 26100；正式签名/Store 包使用证书或 Partner Center 的精确 Publisher，保留 build 19041 最低版本，且不得包含未签名 OID。

上传前，`publish-stage` 会 checkout 当前提交并运行 `tests/smoke/public-release-assets.ps1`，确认 Release 只包含上述四类用户资产、每个负载都有且只有一个校验和、文件非空，并拒绝 DLL、配置、ZIP、SBOM 等额外文件。这样可以让发布工作流的内部产物与公开下载页保持明确边界。

阶段预发布的 workflow 会自动在 Release Notes 中加入 `Free code signing provided by SignPath.io, certificate by SignPath Foundation.`，同时保留 GitHub 自动生成的变更说明。这是基金会署名/申请说明，不代表当前未签名开发包已经获得受信任签名。

手动 Release 可选择 `digicert-stm` 做完整签名演练，并可选择 `require_release_ready` 打开正式门禁。构建阶段要求仓库 Variables `ZIFILE_MSIX_IDENTITY`、`ZIFILE_MSIX_PUBLISHER`、`ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME`；受保护 Environment 提供 Variables `SM_HOST`、`SM_KEYPAIR_ALIAS` 和 Secrets `SM_API_KEY`、`SM_CLIENT_CERT_FILE_B64`、`SM_CLIENT_CERT_PASSWORD`。正式身份只在显式签名/正式门禁演练中传给构建器；普通公开发布保留隔离的 `.Dev` 身份。客户端认证证书只用于登录签名服务，写入 Runner 临时目录并在作业结束前删除；代码签名私钥始终留在云 HSM。

正式标签或 `digicert-stm` 演练会在编译前运行 `Test-PartnerCenterIdentity.ps1 -RequireConfigured`。它要求 Name、Publisher 与 Publisher Display Name 同时存在，Name 符合 MSIX 的 3–50 位字母数字/点/横线边界，Publisher 是有效 X.500 distinguished name，并拒绝 `.Dev` 与未签名 OID；Display Name 必须是 Partner Center 开发者账户中的精确值。三个值必须从 Partner Center 原样复制；预检只证明结构和来源约束，不证明账号或名称已经预留。

生产配置、审批、轮换、应急停止、吊销和最小证据集见[生产签名运维](/zifile/development/signing-operations/)。签名 Job 按架构串行化不同发布运行、设置 30 分钟硬超时，并使用 Job 级最小权限；任何超时都作为失败处理，不能绕过签后验签直接发布。

每次打包都会重新解包 MSIX，并核对 Identity、Publisher、Publisher Display Name、版本、最低 Windows build、桌面/CLI/Worker 三枚 EXE 与 Explorer DLL 的 PE 架构、主要文件关联、`zifile.exe` alias、敏感文件/ZIP 缺失和签名状态。审计还以资源数据方式打开桌面 EXE，不执行目标程序，逐帧验证 16/24/32/48/256 的 `GROUP_ICON`/`ICON` 资源；结果写入 `embedded_desktop_icon`。审计 JSON 随对应架构进入校验和、来源证明和 Release artifact；它不能替代安装、升级、卸载或 WACK 实机门禁。

`tests/smoke/msix-lifecycle.ps1` 为可信签名的基线包和升级包提供显式实机门禁。它先运行包审计并拒绝任何既有同 Identity 安装，然后依次验证安装、包内 CLI、版本升级、`Reset-AppxPackage` 和卸载；无论中途是否失败，都会尝试清理本次测试安装并写出 JSON。微软将 Reset 定义为恢复初始配置，因此脚本不会把它误记为保留数据的 Repair；正式 Repair 仍是独立门禁。参见 [Appx 模块](https://learn.microsoft.com/powershell/module/appx/)与 [Reset-AppxPackage](https://learn.microsoft.com/powershell/module/appx/reset-appxpackage)。

手动 **Trusted MSIX lifecycle** 工作流接收两个 Release 签名运行 ID，从各自的 `signed-windows-x64` artifact 读取默认包和审计 JSON，并在干净 Windows Runner 执行同一门禁、保存 30 天证据。它明确拒绝签名前的 `windows-x64` 产物；ARM64 包的真实安装仍必须在物理 ARM64 Windows 环境完成。

Windows Release 使用仓库固定的 Rust 1.93.0、锁文件、单作业 Cargo 构建和 MSVC `/Brepro` 确定性链接。x64/ARM64 测试与打包 Job 各有 90 分钟硬超时；超时按失败处理，不能跳过包审计或产物上传。独立的双构建工作流会在两个隔离目标目录比较五个裸 PE 文件的 SHA-256；方法与证据边界见[可复现 Windows 构建](/zifile/development/reproducible-builds/)。

在打标签前可从 Actions 手动运行 Release 工作流。该模式不接收第二个版本输入，而是使用 `Cargo.toml` 的工作区版本；推送 tag 会自动公开发布，`none` 构建未签名双架构产物，`digicert-stm` 才进入受保护环境并保存签后产物。推荐每个阶段使用精确递增的带连字符标签，例如 `v0.1.0-alpha.1`、`v0.1.0-beta.1` 和 `v1.0.0-rc.1`；稳定版本使用无连字符的 `v1.0.0`，默认也可发布公开 GitHub 构建，需要正式渠道时再显式启用 `require_release_ready`。普通 CI 与 Release 都运行版本一致性门禁；标签必须精确匹配 `v<workspace-version>`。CLI、核心 Provider 和 IPC 的兼容边界见[公开契约与版本策略](/zifile/development/contracts/)。

普通 CI 还会检查 `CHANGELOG.md` 只有一个 `[Unreleased]` 章节。标签发布必须先把本次内容整理为 `## [<workspace-version>] - YYYY-MM-DD`，至少包含一个 Keep a Changelog 分类和一条非占位更新；版本标题缺失、日期无效、空章节或残留 `TODO`/`TBD` 都会在构建前失败。手动 Release 只验证 `[Unreleased]` 结构，便于发布前演练。

仓库以 [`release/readiness.json`](https://github.com/ax2/zifile/blob/main/release/readiness.json) 跟踪 1.0 的 11 项正式渠道门禁。普通 CI 与公开发布检查结构和证据格式；只有显式启用 `require_release_ready` 时，才会运行 `Test-ReleaseReadiness.ps1 -RequireReleaseReady` 并在任一 `pending` 项时拒绝构建。当前状态为 `candidate`，详见 [1.0 发布就绪状态](/zifile/releases/release-readiness/)。

手动验证模式还会构建 `-accessible` 后缀的 Dioxus/WebView2 候选 MSIX 与完整可运行目录，并将候选桌面程序以规范的 `zifile-desktop.exe` 名称放入包内。正式标签仍只发布当前默认 UI；候选通过 Narrator、Accessibility Insights、IME、DPI 和双架构运行验证后才允许替换默认发行物。

## 发布门禁

只有当单元、互操作、安全、性能、包安装、升级和文档检查全部通过，且 Stage 工作日志已同步，才允许创建稳定版本。`Test-WackReadiness.ps1` 会先无副作用核对当前管理员交互会话、WACK 工具、主机/包架构、schema v2 审计与包哈希、Partner Center 的 Identity/Publisher/Publisher Display Name 精确三元组、build 19041、禁用文件和双重 `Valid` 签名，并可保存失败证据。它不安装包或运行 WACK。WACK CLI 仍必须在当前用户的管理员交互式会话运行；普通权限的自动化终端不能把“预检通过”或“工具已安装”写成“认证已通过”。
