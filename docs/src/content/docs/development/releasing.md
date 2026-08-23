---
title: 构建与发布
description: GitHub、WinGet 与 Microsoft Store 的统一版本发布流程。
---

## 发布产物

- Windows x64 与 ARM64 桌面程序和 CLI。
- MSI 与 MSIX 安装包。
- SHA-256 校验文件。
- SPDX 或 CycloneDX SBOM。
- GitHub 构建来源证明。
- 版本更新日志和对应文档快照。

## 渠道

GitHub Release 是公开构建的第一落点。WinGet manifest 使用计划 ID `ZiCode.ZiFile` 并引用发行方控制的版本化下载地址。Microsoft Store 以 MSIX 为主。

文档当前发布到 `ax2.github.io/zifile`；完成 DNS 配置后迁移到计划域名 `zifile.zicode.com`。

Partner Center 需要先手动预留名称并完成首个提交；之后可以接入 Store Submission API。正式发布工作流使用 GitHub Environment 人工批准，密钥只保存在 GitHub Secrets 或云签名服务中。

## 发布门禁

只有当单元、互操作、安全、性能、包安装、升级和文档检查全部通过，且 Stage 工作日志已同步，才允许创建稳定版本。
