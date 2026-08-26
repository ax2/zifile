---
title: 1.0 发布就绪状态
description: ZiFile 稳定版标签的机器可读门禁与当前证据边界。
---

仓库以 [`release/readiness.json`](https://github.com/ax2/zifile/blob/main/release/readiness.json) 作为 1.0 外部门禁的唯一结构化状态。当前状态是 `candidate`：11 项门禁均保持 `pending`，没有把计划、脚本存在或未签名演练写成通过证据。

## 稳定标签规则

日常 CI、手动 Release 演练和带连字符的预发布标签会验证清单结构。稳定标签还会调用 `Test-ReleaseReadiness.ps1 -RequireReleaseReady`；只要任一门禁仍为 `pending` 就会在构建和发布前失败。状态改为 `passed` 时必须附带本仓库的 Actions、Issue、PR 或 Release 证据链接。

## 当前 11 项门禁

1. 1.0 提交冻结公开契约。
2. 无干扰真实窗口多操作队列。
3. 可信签名 MSIX 安装、启动、关联、升级、Repair 和卸载。
4. 物理 ARM64 Windows 运行。
5. 讲述人、键盘、中文输入法、高对比度与 DPI，并决定默认 UI。
6. 正式签名候选的双语 Store 截图。
7. x64 与 ARM64 正式 WACK 报告。
8. Microsoft Store 提交与认证。
9. WinGet 社区仓库接受。
10. Partner Center 名称与正式 Package Identity。
11. ADR-0006 已移除 PFX 并接入云 HSM 签名/签后审计；仍需真实证书完成双架构签名、可信生命周期和吊销演练。

此页面是可读说明，JSON 清单和机器门禁才决定稳定标签是否允许。`candidate` 不等于 Store-ready、已签名或可发布。
