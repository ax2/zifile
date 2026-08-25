---
title: ADR-0006：Windows 发行签名
description: Store、GitHub 与 WinGet 的受信任签名路线和凭据边界。
---

## 状态

接受，2026-08-25。外部账号尚未开通，因此本 ADR 决定架构和供应商优先级，不宣称已经取得证书或完成可信签名。

## 背景

ZiFile 同时面向 Microsoft Store、GitHub Release 和 WinGet。GitHub 来源证明可以证明构建来源，但不能替代 Windows Authenticode 信任。把可导出的长期 PFX 和密码存入 GitHub Secrets 也不再作为新证书的产品路线。

Microsoft Store 接收与 Partner Center Identity 匹配的 MSIX，并在认证流程中签名。GitHub/WinGet 直发包则需要发布者自己的公开受信任签名、SHA-256 时间戳和签名后审计。

## 决策

- Store 通道提交未由 ZiFile 生产证书签名、但 Identity/Publisher 与 Partner Center 精确匹配的 MSIX，由 Store 完成最终签名。
- GitHub/WinGet 通道使用云 HSM 托管的组织验证代码签名；私钥不得导出到仓库、开发机或 GitHub Secret。
- 若 ZiCode 的签约主体满足 Microsoft Artifact Signing Public Trust 的区域与身份要求，首选 Artifact Signing，并使用 GitHub OIDC/最小权限服务身份和 Microsoft 时间戳服务。
- 在主体资格未确认或不满足时，落地基线为 DigiCert Software Trust Manager 的公开受信任 OV 证书、KSP/SignTool 集成和 GitHub Actions 短期认证。采购前仍需核对主体可验证性、地区销售、价格和签名额度。
- 当前 PFX workflow 仅是既有管线脚手架，不得用于 1.0 标签；云签名接入、签名后重新审计、可信安装升级和撤销演练完成后才能解除发布门禁。

## 验收条件

签名集成必须在 x64 与 ARM64 上签署独立 EXE、Shell DLL 和 MSIX，使用 SHA-256 文件摘要与 RFC 3161 时间戳；随后以系统信任链验证签名，并重新生成包审计、校验和与来源证明。日志不得打印认证材料，生产签名只允许受保护的 tag/environment，且必须保留最小权限、审批、轮换、吊销与应急停止说明。

Artifact Signing 当前只向美国、加拿大、欧盟和英国的组织开放 Public Trust（个人范围更窄），所以没有主体资格证据时不能假定可用。DigiCert 只是当前可实施基线，不构成采购授权；若商务核验失败，可替换为满足同等公开信任、HSM 和自动化要求的 CA 云签名服务。
