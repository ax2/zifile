---
title: ADR-0006：Windows 发行签名
description: Store、GitHub 与 WinGet 的受信任签名路线和凭据边界。
---

## 状态

接受，2026-08-25；实现更新于 2026-08-26。外部账号尚未开通，因此本 ADR 决定架构、供应商优先级和 CI 接口，不宣称已经取得证书或完成可信签名。

## 背景

ZiFile 同时面向 Microsoft Store、GitHub Release 和 WinGet。GitHub 来源证明可以证明构建来源，但不能替代 Windows Authenticode 信任。把可导出的长期 PFX 和密码存入 GitHub Secrets 也不再作为新证书的产品路线。

Microsoft Store 接收与 Partner Center Identity 匹配的 MSIX，并在认证流程中签名。GitHub/WinGet 直发包则需要发布者自己的公开受信任签名、SHA-256 时间戳和签名后审计。

## 决策

- Store 通道提交未由 ZiFile 生产证书签名、但 Identity/Publisher 与 Partner Center 精确匹配的 MSIX，由 Store 完成最终签名。
- GitHub/WinGet 通道使用云 HSM 托管的组织验证代码签名；私钥不得导出到仓库、开发机或 GitHub Secret。
- 若 ZiCode 的签约主体满足 [Microsoft Artifact Signing Public Trust](https://learn.microsoft.com/azure/artifact-signing/quickstart) 的区域与身份要求，长期首选 Artifact Signing，并使用 GitHub OIDC/最小权限服务身份和 Microsoft 时间戳服务。官方 Action 当前名称为 `Azure/artifact-signing-action`，旧 Trusted Signing 名称不得继续使用。
- 在主体资格尚未确认时，落地基线为 DigiCert Binary Signing 的公开受信任组织验证证书和官方 `digicert/code-signing-software-trust-action`。新集成使用官方推荐的 simple signing，不依赖已进入退役路线的旧 KSP/SignTool GitHub Action；采购前仍需核对主体可验证性、地区销售、价格、签名额度和 simple-signing 权限。
- PFX 已从 Release workflow 移除。云签名的无凭据 CI 接口、签后重新审计和签后来源证明已经落地；真实账号签名、可信安装升级和吊销演练完成后才能解除发布门禁。

## 成本、续期与运维比较

| 方案 | 当前公开成本基线 | CI 与密钥保管 | 续期/主要限制 |
| --- | --- | --- | --- |
| Artifact Signing Basic | USD 9.99/月，含 5,000 次签名，超额 USD 0.005/次 | 官方 GitHub Action；服务端 HSM 与三日短期证书，必须时间戳 | 证书生命周期自动管理；Public Trust 有主体地区限制 |
| DigiCert Binary Signing | 以正式报价为准 | simple signing Action；私钥留在云 HSM，CI 使用 API 与客户端认证材料 | 订阅和组织验证需续期；额度、地区销售与自动化权限需在采购前书面确认 |

Artifact Signing 价格是 2026-08-25 的公开页面快照，只用于路线选择，不是采购报价；DigiCert 不在文档中固化未经合同确认的价格。x64 与 ARM64 都使用同一发布者身份；签名工具对文件格式工作，不要求在 ARM64 主机上完成签名，但两种包都必须分别验签。

## 验收条件

签名集成必须在 x64 与 ARM64 上签署独立 EXE、Shell DLL 和 MSIX，使用 SHA-256 文件摘要与 RFC 3161 时间戳；随后以系统信任链验证签名，并重新生成包审计、校验和与来源证明。日志不得打印认证材料，生产签名只允许受保护的 tag/environment，且必须保留最小权限、审批、轮换、吊销与应急停止说明。

[Microsoft 当前清单](https://learn.microsoft.com/azure/artifact-signing/quickstart)列出的 Public Trust 组织地区包括美国、加拿大、欧盟、英国、澳大利亚、新西兰、日本、韩国、新加坡、瑞士、挪威和以色列；个人开发者范围仍更窄。资源区域与主体资格是不同约束，没有 ZiCode 法律主体资格证据时不能假定可用。DigiCert 只是当前可实施基线，不构成采购授权；若商务核验失败，可替换为满足同等公开信任、HSM、时间戳和自动化要求的 CA 云签名服务。

## CI 凭据边界

`production-signing` GitHub Environment 保存审批和生产范围。构建阶段需要的 `ZIFILE_MSIX_IDENTITY` 与 `ZIFILE_MSIX_PUBLISHER` 是仓库级非秘密变量；只在签名阶段需要的 `SM_HOST` 与 `SM_KEYPAIR_ALIAS` 是 Environment 非秘密变量。`SM_API_KEY`、客户端认证证书的 Base64 和密码是 Environment Secret。客户端认证证书只用于访问服务，不是可导出的代码签名私钥，并在 Runner 临时目录使用后删除。标签必须走云签名；手动 Release 可选择 `none` 做无签名构建，或选择 `digicert-stm` 做受保护的真实签名演练。
