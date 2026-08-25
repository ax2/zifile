---
title: Microsoft Store 与 WinGet 发布清单
description: ZiFile 的外部分发门禁、所需材料与可重复命令。
---

## 自动化已具备

- x64 与 ARM64 的完整可运行目录、独立 EXE 和 MSIX 构建。
- 可选 PFX 签名，证书只通过 GitHub Actions Secret 注入并在作业结束前删除。
- SHA-256、CycloneDX JSON SBOM 和 GitHub 构建来源证明。
- WinGet 1.12 多文件清单生成器，覆盖两个架构、文件关联和中英文元数据。
- 真实 Release SHA-256 的候选清单已通过本机 `winget validate`；该结果不等于已提交或已获社区仓库接受。
- 每个 MSIX 在构建后自动解包审计 Identity、Publisher、版本、最低系统版本、四枚 PE 架构（桌面、CLI、Worker 与 Explorer DLL）、主要文件关联、CLI alias、敏感文件和签名状态，并随包生成 `.audit.json`。
- 标签发布缺少正式 Identity、Publisher、PFX 或密码时会在打包前失败；`.Dev` Identity 和未签名 OID Publisher 不能用于标签发布。
- 简体中文和英文 Store 文案、隐私说明与认证备注已经结构化归档；CI 验证描述、功能、关键词、许可与 HTTPS URL 的 Partner Center 字段限制。

## 开发包与签名边界

未签名 `.Dev` 包使用微软规定的固定 Publisher OID；构建脚本拒绝缺少该 OID、使用任意替代 OID，或签名时继续使用未签名命名空间。该开发路径将最低系统版本提升到 Windows 11 build 26100，正式签名包和 Store 包仍保持 build 19041。当前测试机的部署解析器仍以 `0x80080204` 拒绝该 OID Publisher，因此未签名安装尚未通过。

一次临时自签名演练证明 manifest Publisher、证书 subject 与 SignTool 链路一致；安装按预期停在不受信任根 `0x800B0109`。测试没有导入根证书，也没有保留私钥、证书或包注册。正式门禁必须使用可信证书或 Partner Center Identity，不能用这项演练替代。

## 首次上架前的外部门禁

1. 在 Partner Center 注册 Windows 开发者账号并预留 `ZiFile` 名称。
2. 将 Partner Center 分配的 Package Identity Name 与 Publisher 写入 GitHub Secrets。
3. 准备可信代码签名证书用于 GitHub/WinGet；Store 分发包由 Microsoft Store 签名。
4. 用正式 Identity 重建 x64 与 ARM64 包，分别验证安装、启动、文件关联、升级和卸载。
5. 在当前用户的管理员交互式会话中运行 Windows App Certification Kit，并完成键盘、讲述人、高对比度、DPI 与中文输入法检查。
6. 复核已准备的双语商店说明、隐私说明和认证备注，部署公开隐私页，采集正式候选包的双语桌面截图，并填写年龄分级与市场。
7. 上传通过验证的 MSIX 包并提交认证；公开 Release 后再生成和验证 WinGet 清单 PR。

未完成这些外部门禁时，任何 Alpha 构建都不得标记为“Microsoft Store 已就绪”或“已签名”。

微软参考资料：[WinGet 1.12 多文件清单规范](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.12.0)、[Microsoft Store 提交流程](https://learn.microsoft.com/windows/apps/publish/faq/submit-your-app)、[MSIX 包要求](https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/app-package-requirements)。
