---
title: Microsoft Store 与 WinGet 发布清单
description: ZiFile 的外部分发门禁、所需材料与可重复命令。
---

## 自动化已具备

- x64 与 ARM64 的完整可运行目录、独立 EXE 和 MSIX 构建。
- 可选 PFX 签名，证书只通过 GitHub Actions Secret 注入并在作业结束前删除。
- SHA-256、CycloneDX JSON SBOM 和 GitHub 构建来源证明。
- WinGet 1.12 多文件清单生成器，覆盖两个架构、文件关联和中英文元数据。

## 首次上架前的外部门禁

1. 在 Partner Center 注册 Windows 开发者账号并预留 `ZiFile` 名称。
2. 将 Partner Center 分配的 Package Identity Name 与 Publisher 写入 GitHub Secrets。
3. 准备可信代码签名证书用于 GitHub/WinGet；Store 分发包由 Microsoft Store 签名。
4. 用正式 Identity 重建 x64 与 ARM64 包，分别验证安装、启动、文件关联、升级和卸载。
5. 运行 Windows App Certification Kit，并完成键盘、讲述人、高对比度、DPI 与中文输入法检查。
6. 准备商店说明、隐私说明、图标和桌面截图，填写年龄分级、市场与认证备注。
7. 上传通过验证的 MSIX 包并提交认证；公开 Release 后再生成和验证 WinGet 清单 PR。

未完成这些外部门禁时，任何 Alpha 构建都不得标记为“Microsoft Store 已就绪”或“已签名”。

微软参考资料：[WinGet 1.12 多文件清单规范](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.12.0)、[Microsoft Store 提交流程](https://learn.microsoft.com/windows/apps/publish/faq/submit-your-app)、[MSIX 包要求](https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/app-package-requirements)。
