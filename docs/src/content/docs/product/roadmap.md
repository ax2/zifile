---
title: 路线图
description: ZiFile 从基础验证到 1.0 的阶段计划。
---

根目录 [`ROADMAP.md`](https://github.com/ax2/zifile/blob/main/ROADMAP.md) 是路线图的唯一权威来源。本页提供面向读者的摘要。

| 阶段 | 目标 | 关键交付 |
| --- | --- | --- |
| Stage 0 | 验证基础 | Rust workspace、Iced、CI、Starlight、ADR、十万条目有界列表验证 |
| Stage 1 | Alpha | 已打通 ZIP/7z/TAR 家族、固定 MSZIP 创建的 Beta 级 Windows CAB、Beta 级只读 RAR 1.3–7、安全解压、中英文 UI、搜索分页、真实进度、取消、全格式 parser fuzz，以及 7-Zip/RAR/CAB 参考语料；队列的真实前台回合仍属于发布门禁 |
| Stage 2 | Beta | 文件关联、任务栏、App Execution Alias、隔离 Worker、双架构包、十万项浏览/取消基线，以及创建/解压到同名目录右键命令均已实现；可信签名安装、升级和真实 Explorer 激活仍属于发布门禁 |
| Stage 3 | RC | Dioxus/WebView2 语义候选已打通主要 Worker 流程、CSP、核心快捷键、换页后带标题的主区域焦点、中英文导航/创建表单键盘回归、双语关于/运行版本页、双架构候选包、完整高 DPI MSIX 图标矩阵、16/24/32/48/256 多分辨率 Win32 图标、双语文档、机器校验的 Store 文案、截图原子导入、WACK readiness，以及无 PFX 的受保护云签名/签后审计链路；继续前台焦点/Narrator/Accessibility Insights、ARM64 实机、真实签名、正式截图、WinGet 与 Store 验证 |
| Stage 4（进行中） | 1.0 | API 冻结、文档完成、三渠道正式发布 |

每个阶段必须有独立工作日志，记录目标、发现、修改、验证、遗留问题和发布结果。

GitHub Milestones 与权威路线图同步：Stage 1 队列为 [#11](https://github.com/ax2/zifile/issues/11)，Stage 2 可信安装/Explorer 与 ARM64 为 [#12](https://github.com/ax2/zifile/issues/12)–[#13](https://github.com/ax2/zifile/issues/13)，Stage 3 辅助功能、正式截图、WACK、Store 与 WinGet 为 [#14](https://github.com/ax2/zifile/issues/14)–[#18](https://github.com/ax2/zifile/issues/18)，三渠道 1.0 为 [#19](https://github.com/ax2/zifile/issues/19)。Partner Center 与签名继续由 [#8](https://github.com/ax2/zifile/issues/8)–[#9](https://github.com/ax2/zifile/issues/9) 跟踪。
