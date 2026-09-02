---
title: 代码签名政策
description: ZiFile 的 SignPath Foundation 申请、签名角色、构建来源和发布通道政策。
---

## 当前状态

ZiFile 正在准备申请 SignPath Foundation 的开源免费代码签名服务。申请获批前，仓库不会把任何构建产物描述为 SignPath Foundation 已签名，也不会把申请中的证书接入正式发布门禁。

申请服务的固定说明是：

> Free code signing provided by SignPath.io, certificate by SignPath Foundation.

完整的公开政策位于仓库根目录的 [`CODE-SIGNING-POLICY.md`](https://github.com/ax2/zifile/blob/main/CODE-SIGNING-POLICY.md)。

## 角色与审批

- Committers 与 reviewers：[@ax2](https://github.com/ax2)，当前仓库所有者和 CODEOWNERS 成员。
- Release approver：[@ax2](https://github.com/ax2)。

源代码、构建脚本、打包配置和签名配置必须通过 GitHub Pull Request 审查。签名审批人检查源提交、版本、架构、产物范围、来源证明和签后验证结果后，才能批准签名请求。新增维护者必须先更新本页面和根目录政策，再获得签名权限。

GitHub 和签名服务账号启用多因素认证。签名凭据、私钥和令牌不会写入仓库、Release、Issue、PR、文档档案或普通构建产物。

## 构建与发布边界

只有由本仓库审查过的源代码和构建配置产生的 ZiFile 二进制文件可以提交签名。GitHub Actions 是发布构建的权威路径；流程保留源提交、架构、版本、来源证明、包审计和 SHA-256，并在签名后再次验证。

MSIX 的 `Identity Publisher` 必须与签名证书主体一致。Store 使用 Partner Center 的独立包身份，不能未经评审就用另一种证书主体替换。WinGet 清单使用签名后的最终文件计算哈希，不能使用签名前的构建文件。

## 隐私

ZiFile 不会把用户文件、压缩包内容、路径或密码发送给签名服务或 ZiCode。[隐私说明](/zifile/product/privacy/)描述了应用本身的数据边界。

> This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.

## 申请入口

[SignPath Foundation 免费申请](https://signpath.org/apply.html)
