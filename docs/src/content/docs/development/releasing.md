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

GitHub Release 是公开构建的第一落点。WinGet manifest 使用计划 ID `ZiCode.ZiFile` 并引用发行方控制的版本化下载地址。Microsoft Store 以 MSIX 为主。

文档当前发布到 `ax2.github.io/zifile`；完成 DNS 配置后迁移到计划域名 `zifile.zicode.com`。

Partner Center 需要先手动预留名称并完成首个提交；之后可以接入 Store Submission API。签名策略见 [ADR-0006](/zifile/architecture/adr-0006-release-signing/)：Store 完成商店通道最终签名，GitHub/WinGet 使用云 HSM 托管的公开受信任签名，生产私钥不得导出到 GitHub Secret。现有 PFX 路径仅为管线脚手架，不能解除 1.0 发布门禁。

推送 `v*` 标签会为 x64 和 ARM64 构建 MSIX 与独立 EXE，生成校验和、结构化包审计、CycloneDX SBOM、来源证明和 WinGet 1.12 多文件清单候选，然后发布 GitHub Release。当前预发布标签流程要求正式 Identity、Publisher、PFX 和密码四项 Secret 全部存在；缺一项、使用 `.Dev` Identity 或未签名 OID Publisher 都会在构建前失败。稳定版标签还会拒绝 PFX 脚手架，直到 ADR-0006 云 HSM 集成落地，避免误发不符合签名决策的 1.0。没有正式凭据时只能手动生成开发用途的未签名包，不得提交 WinGet 或 Store。未签名 `.Dev` 包使用微软固定 OID Publisher并要求 Windows 11 build 26100；正式签名/Store 包使用证书或 Partner Center 的精确 Publisher，保留 build 19041 最低版本，且不得包含未签名 OID。

每次打包都会重新解包 MSIX，并核对 Identity、Publisher、版本、最低 Windows build、桌面/CLI/Worker 三枚 EXE 与 Explorer DLL 的 PE 架构、主要文件关联、`zifile.exe` alias、敏感文件/ZIP 缺失和签名状态。审计 JSON 随对应架构进入校验和、来源证明和 Release artifact；它不能替代安装、升级、卸载或 WACK 实机门禁。

`tests/smoke/msix-lifecycle.ps1` 为可信签名的基线包和升级包提供显式实机门禁。它先运行包审计并拒绝任何既有同 Identity 安装，然后依次验证安装、包内 CLI、版本升级、`Reset-AppxPackage` 和卸载；无论中途是否失败，都会尝试清理本次测试安装并写出 JSON。微软将 Reset 定义为恢复初始配置，因此脚本不会把它误记为保留数据的 Repair；正式 Repair 仍是独立门禁。参见 [Appx 模块](https://learn.microsoft.com/powershell/module/appx/)与 [Reset-AppxPackage](https://learn.microsoft.com/powershell/module/appx/reset-appxpackage)。

手动 **Trusted MSIX lifecycle** 工作流接收两个 Release 验证运行 ID，从各自的 `windows-x64` artifact 读取默认包和审计 JSON，并在干净 Windows Runner 执行同一门禁、保存 30 天证据。ARM64 包的真实安装仍必须在物理 ARM64 Windows 环境完成。

Windows Release 使用仓库固定的 Rust 1.93.0、锁文件、单作业 Cargo 构建和 MSVC `/Brepro` 确定性链接。独立的双构建工作流会在两个隔离目标目录比较五个裸 PE 文件的 SHA-256；方法与证据边界见[可复现 Windows 构建](/zifile/development/reproducible-builds/)。

在打标签前可从 Actions 手动运行 Release 工作流。该模式不接收第二个版本输入，而是使用 `Cargo.toml` 的工作区版本，真实构建和保存双架构产物与 SBOM，但会跳过公开 Release 和 WinGet 发布候选，适合验证交叉编译与打包环境。普通 CI 与 Release 都运行版本一致性门禁；标签必须精确匹配 `v<workspace-version>`。CLI、核心 Provider 和 IPC 的兼容边界见[公开契约与版本策略](/zifile/development/contracts/)。

手动验证模式还会构建 `-accessible` 后缀的 Dioxus/WebView2 候选 MSIX 与完整可运行目录，并将候选桌面程序以规范的 `zifile-desktop.exe` 名称放入包内。正式标签仍只发布当前默认 UI；候选通过 Narrator、Accessibility Insights、IME、DPI 和双架构运行验证后才允许替换默认发行物。

## 发布门禁

只有当单元、互操作、安全、性能、包安装、升级和文档检查全部通过，且 Stage 工作日志已同步，才允许创建稳定版本。`Test-WackReadiness.ps1` 会先无副作用核对当前管理员交互会话、WACK 工具、主机/包架构、schema v2 审计与包哈希、正式 Identity/Publisher、build 19041、禁用文件和双重 `Valid` 签名，并可保存失败证据。它不安装包或运行 WACK。WACK CLI 仍必须在当前用户的管理员交互式会话运行；普通权限的自动化终端不能把“预检通过”或“工具已安装”写成“认证已通过”。
