---
title: 生产签名运维
description: DigiCert Binary Signing 的配置、审批、演练、轮换、吊销与证据保留手册。
---

## 当前边界

本手册执行 [ADR-0006](/zifile/architecture/adr-0006-release-signing/) 的 DigiCert Binary Signing 基线。仓库已经具备受保护的 `production-signing` Environment、云签名 Action 和签后验证器，但尚未配置组织账号、公开受信任证书或凭据。没有真实运行证据前，`production-cloud-hsm-signing` 必须保持 `pending`。

代码签名私钥必须留在云 HSM。`SM_CLIENT_CERT_FILE_B64` 是访问服务的客户端认证证书，不是代码签名私钥；它仍属于 Secret，不能写入仓库、日志、Issue、PR、档案或本地长期文件。

## 配置清单

| 位置 | 名称 | 性质 | 用途 |
| --- | --- | --- | --- |
| Repository Variable | `ZIFILE_MSIX_IDENTITY` | 非秘密 | Partner Center 分配的精确 Package Identity Name |
| Repository Variable | `ZIFILE_MSIX_PUBLISHER` | 非秘密 | 与证书 Subject、MSIX Publisher 精确一致 |
| Repository Variable | `ZIFILE_MSIX_PUBLISHER_DISPLAY_NAME` | 非秘密 | Partner Center 开发者账户的精确 Publisher Display Name |
| Environment Variable | `SM_HOST` | 非秘密 | DigiCert 服务端点 |
| Environment Variable | `SM_KEYPAIR_ALIAS` | 非秘密 | 获准用于 ZiFile 生产发布的 Keypair Alias |
| Environment Secret | `SM_API_KEY` | 秘密 | 最小权限自动化 API Key |
| Environment Secret | `SM_CLIENT_CERT_FILE_B64` | 秘密 | 客户端认证 PKCS#12 的 Base64 |
| Environment Secret | `SM_CLIENT_CERT_PASSWORD` | 秘密 | 客户端认证证书密码 |

首次配置必须完成组织验证、证书用途/额度确认、最小权限服务用户和审计日志启用。`production-signing` 保持 required reviewer；部署策略只允许 `v*` 标签和明确的演练分支。不得为了排查失败临时打印 Secret、放宽到所有分支或改用可导出 PFX。

## 首次真实演练

1. 在 Release workflow 手动选择 `signing_provider=digicert-stm`，核对源提交与工作区版本后批准 Environment deployment。
2. 要求 x64、ARM64 两个 `Sign Windows` Job 均成功；任何缺参、Action 非零退出、无效签名、无时间戳、Publisher 或 Publisher Display Name 不一致都必须终止。
3. 下载 `signed-windows-x64` 与 `signed-windows-arm64`。核对 `.signing.json` 的五项签名、`.audit.json` 的 `Valid` 状态、`SHA256SUMS-*` 和 GitHub provenance；不得保留 artifact ZIP，档案只保存解包后的证据或完整可运行目录。
4. 在干净 x64 机器执行可信安装、启动、Explorer、升级、Repair/Reset 和卸载门禁；ARM64 在物理 Windows ARM64 设备重复。随后执行 WACK readiness 与正式 WACK。
5. 只有双架构签名、生命周期、吊销演练和证据 URL 都通过，才可更新 `release/readiness.json`；CI 接线本身不能解除门禁。

## 正式发布

发布者先冻结版本、CHANGELOG、Store 截图和 11 项 readiness 证据，再创建精确 `v<workspace-version>` 标签。审核人必须核对提交、版本、证书主体、Keypair Alias 和变更范围后批准。发布完成后重新下载 GitHub Release 文件，复核公开 SHA-256、签名、时间戳、来源证明、WinGet 清单哈希和 Release 附件中不存在 ZIP 或认证材料。

## 轮换

- API Key 与客户端认证证书分开轮换。先创建最小权限新凭据、更新 Environment Secret、执行一次 `digicert-stm` 演练并验签，再撤销旧凭据；档案只记录日期、责任人、运行 URL 和结果。
- 代码签名证书或 Keypair 轮换前确认新 Subject 与 `ZIFILE_MSIX_PUBLISHER` 精确一致。用新证书完成双架构演练和升级测试后再切换正式发布；旧的已时间戳签名证据继续保留。
- 定期检查服务用户、Environment reviewer、部署分支、签名额度和审计日志。离职、权限变化或供应商告警必须触发即时复核。

## 应急停止与吊销

怀疑 API Key、客户端认证材料、服务账号、Keypair 或发布产物被滥用时，立即执行 emergency stop：取消进行中的 Release，禁用 `production-signing` Environment 或移除允许策略，撤销 API Key/客户端认证证书并在 DigiCert 禁用 Keypair。不要等待根因分析后才停止签名。

随后保存非敏感审计日志和受影响哈希，核对 GitHub Actions、DigiCert 签名日志和 Release 下载；删除或明确标记受影响的未验证 Release，评估证书吊销与用户通知。新凭据和 Keypair 必须完成完整演练后才能恢复 Environment。吊销演练应使用供应商测试流程或专用测试证书，不能为了演练吊销生产证书。

## 最小证据集

每次生产演练或发布保留：源提交/tree、workflow/job URL、架构、版本、Identity、Publisher、Publisher Display Name、signer/timestamp thumbprint、五个文件 SHA-256、签名 JSON、包审计、provenance、生命周期/WACK 结果和 reviewer。不得保留 API Key、密码、客户端证书内容、Cookie、令牌或代码签名私钥。
