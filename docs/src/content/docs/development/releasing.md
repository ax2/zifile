---
title: 构建与发布
description: GitHub、WinGet 与 Microsoft Store 的统一版本发布流程。
---

## 发布产物

- Windows x64 与 ARM64 桌面程序和 CLI。
- MSIX 安装包与无需安装的独立 EXE。
- SHA-256 校验文件。
- CycloneDX JSON SBOM。
- GitHub 构建来源证明。
- 版本更新日志和对应文档快照。

## 渠道

GitHub Release 是公开构建的第一落点。WinGet manifest 使用计划 ID `ZiCode.ZiFile` 并引用发行方控制的版本化下载地址。Microsoft Store 以 MSIX 为主。

文档当前发布到 `ax2.github.io/zifile`；完成 DNS 配置后迁移到计划域名 `zifile.zicode.com`。

Partner Center 需要先手动预留名称并完成首个提交；之后可以接入 Store Submission API。签名密钥只保存在 GitHub Secrets 或云签名服务中，工作流把临时证书写到 Runner 临时目录并在打包后删除。

推送 `v*` 标签会为 x64 和 ARM64 构建 MSIX 与独立 EXE，生成校验和、CycloneDX SBOM、来源证明和 WinGet 1.12 多文件清单候选，然后发布 GitHub Release。没有正式 Identity 和签名 Secret 时只能生成开发用途的未签名包，不得提交 WinGet 或 Store。未签名 `.Dev` 包使用微软固定 OID Publisher 并要求 Windows 11 build 26100；正式签名/Store 包使用证书或 Partner Center 的精确 Publisher，保留 build 19041 最低版本，且不得包含未签名 OID。

在打标签前可从 Actions 手动运行 Release 工作流并填写语义版本。该模式真实构建和保存双架构产物与 SBOM，但会跳过公开 Release 和 WinGet 发布候选，适合验证交叉编译与打包环境。

手动验证模式还会构建 `-accessible` 后缀的 Dioxus/WebView2 候选 MSIX 与完整可运行目录，并将候选桌面程序以规范的 `zifile-desktop.exe` 名称放入包内。正式标签仍只发布当前默认 UI；候选通过 Narrator、Accessibility Insights、IME、DPI 和双架构运行验证后才允许替换默认发行物。

## 发布门禁

只有当单元、互操作、安全、性能、包安装、升级和文档检查全部通过，且 Stage 工作日志已同步，才允许创建稳定版本。WACK CLI 必须在当前用户的管理员交互式会话运行；普通权限的自动化终端不能把“工具已安装”写成“认证已通过”。
