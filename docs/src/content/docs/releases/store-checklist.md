---
title: Microsoft Store 与 WinGet 发布清单
description: ZiFile 的外部分发门禁、所需材料与可重复命令。
---

## 自动化已具备

- x64 与 ARM64 的完整可运行目录、独立 EXE 和 MSIX 构建。
- Release 已移除 PFX，具备受保护的 DigiCert Binary Signing simple-signing、签后系统验签、时间戳检查和签后审计路径；真实账号演练仍待外部凭据。
- `Test-PartnerCenterIdentity.ps1` 在标签/真实签名演练编译前拒绝缺失、只配置一项、非法 Name、`.Dev`、未签名 OID 和无效 X.500 Publisher；正式值仍必须由 Product identity 页面提供。
- SHA-256、CycloneDX JSON SBOM 和 GitHub 构建来源证明。
- WinGet 1.12 多文件清单生成器，覆盖两个架构、文件关联和中英文元数据，并直接生成社区仓库要求的 `manifests/z/ZiCode/ZiFile/<version>/` 目录。
- 标签发布在上传前运行 `Test-Manifests.ps1`，要求四个 schema 类型、正式 GitHub Release HTTPS URL、x64/ARM64 一一对应，并把两个清单 SHA-256 与签后本地 MSIX 精确比较；篡改或错误版本路径会使发布失败。
- Windows CI 还会生成确定性候选并运行系统 `winget validate`，防止自有预检与官方 schema 漂移；该夹具不下载公开包，因此不能作为 Release URL 可用或仓库接受证据。
- 真实 Release SHA-256 的候选清单已通过本机 `winget validate`；该结果不等于已提交或已获社区仓库接受。
- 每个 MSIX 在构建后自动解包审计 Identity、Publisher、版本、最低系统版本、四枚 PE 架构（桌面、CLI、Worker 与 Explorer DLL）、主要文件关联、CLI alias、敏感文件和签名状态，并随包生成 `.audit.json`。
- 标签发布固定进入 `production-signing` Environment，并且发布任务只消费通过签后门禁的 `signed-windows-*`；`.Dev` Identity、未签名 OID Publisher、缺失云签名输入或无效/无时间戳签名都会失败。
- 简体中文和英文 Store 文案、隐私说明与认证备注已经结构化归档；CI 验证描述、功能、关键词、许可与 HTTPS URL 的 Partner Center 字段限制，并拒绝 JSON 与可读文档的描述段落或功能项漂移。
- 隐私 URL 固定指向 GitHub Pages 的中英文政策路由。普通 CI 在 Astro 构建后核对两个 `index.html` 与隐私正文标记；Pages 部署后再次请求公开 HTTPS 页面并要求 HTTP 200，防止 Store listing 引用 404 或错误页面。
- 双语 Desktop 截图清单具有显式 `draft/complete` 状态；门禁验证 PNG/IHDR、1366×768（或纵向等价）最低尺寸、50 MB 上限、SHA-256、路径边界、顺序、场景和 200 字符说明。标签发布必须达到每语言至少四张，否则在打包前失败。
- Store 冒烟会动态生成中英文各四张真实 PNG，证明完整清单通过，并证明低分辨率和重复图片被拒绝；当前仓库清单仍明确为 0 张 `draft`，没有用测试图冒充正式素材。
- `Import-Screenshots.ps1` 从固定的双语四场景采集目录原子导入素材，自动计算哈希和说明，并要求应用版本、Windows build、主题、缩放、UTC 时间、源提交及签名候选类型。它只接受 draft 目标且拒绝覆盖已有 `assets`；任一图片或元数据失败都不会形成 complete 清单。

## 开发包与签名边界

未签名 `.Dev` 包使用微软规定的固定 Publisher OID；构建脚本拒绝缺少该 OID、使用任意替代 OID，或签名时继续使用未签名命名空间。该开发路径将最低系统版本提升到 Windows 11 build 26100，正式签名包和 Store 包仍保持 build 19041。当前测试机的部署解析器仍以 `0x80080204` 拒绝该 OID Publisher，因此未签名安装尚未通过。

一次临时自签名演练证明 manifest Publisher、证书 subject 与 SignTool 链路一致；安装按预期停在不受信任根 `0x800B0109`。测试没有导入根证书，也没有保留私钥、证书或包注册。正式门禁必须使用可信证书或 Partner Center Identity，不能用这项演练替代。

可信生命周期工作流还会构建固定 Windows App SDK 1.8 的自包含测试辅助程序。系统支持 Repair 时，它在升级后调用与 Windows 设置“修复”相同的 `RepairPackageAsync`，并要求包 LocalState 哨兵保持；随后 Reset 必须删除该哨兵。当前 Windows 25H2 build 26200 的无副作用探测返回 `repair_supported=false`，因此只记录能力缺失，不把 Reset 冒充 Repair。

## 首次上架前的外部门禁

这些项目同时记录在机器可读的 [`release/readiness.json`](https://github.com/ax2/zifile/blob/main/release/readiness.json) 和 [1.0 发布就绪状态](/zifile/releases/release-readiness/) 中；稳定标签要求全部有证据并标记为 `passed`。

1. 在 Partner Center 注册 Windows 开发者账号并预留 `ZiFile` 名称。
2. 将 Partner Center 分配的 Package Identity Name 与 Publisher 写入 GitHub Repository Variables。
3. 完成 DigiCert 组织验证与代码签名证书配置，在受保护 Environment 配置 Host、Keypair Alias、API Key 和客户端认证材料，执行双架构手动签名演练；Store 分发包由 Microsoft Store 签名。
4. 用正式 Identity 重建 x64 与 ARM64 包，分别验证安装、启动、文件关联、升级和卸载。
5. 在当前用户的管理员交互式会话中运行 Windows App Certification Kit，并完成键盘、讲述人、高对比度、DPI 与中文输入法检查。
6. 复核已准备的双语商店说明、隐私说明和认证备注，部署公开隐私页，采集正式候选包的双语桌面截图，并填写年龄分级与市场。
7. 上传通过验证的 MSIX 包并提交认证；公开 Release 附带已通过本地签后一致性门禁的 WinGet 清单，再运行官方 `winget validate` 并提交社区仓库 PR。

未完成这些外部门禁时，任何 Alpha 构建都不得标记为“Microsoft Store 已就绪”或“已签名”。

运行 WACK 前先对目标 MSIX 与同目录 `.audit.json` 执行 `Test-WackReadiness.ps1 -RequireReady`。预检通过只表示工具、会话、架构、身份、哈希、最低系统版本和签名满足启动条件；最终仍以 WACK 生成的正式报告为准。

微软参考资料：[WinGet 1.12 多文件清单规范](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.12.0)、[Microsoft Store 提交流程](https://learn.microsoft.com/windows/apps/publish/faq/submit-your-app)、[MSIX 包要求](https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/app-package-requirements)。
