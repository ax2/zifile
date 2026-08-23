---
title: 路线图
description: ZiFile 从基础验证到 1.0 的阶段计划。
---

根目录 [`ROADMAP.md`](https://github.com/ax2/zifile/blob/main/ROADMAP.md) 是路线图的唯一权威来源。本页提供面向读者的摘要。

| 阶段 | 目标 | 关键交付 |
| --- | --- | --- |
| Stage 0 | 验证基础 | Rust workspace、Iced、CI、Starlight、ADR、十万条目有界列表验证 |
| Stage 1（进行中） | Alpha | 已打通 ZIP/7z/TAR 家族、安全解压、中英文 UI、搜索分页、真实进度与取消；继续扩展互操作测试 |
| Stage 2 | Beta | 文件关联、任务栏、App Execution Alias、隔离 Worker 和双架构包已完成；继续签名安装升级、右键命令和系统级性能验证 |
| Stage 3 | RC | Dioxus/WebView2 语义候选已打通主要 Worker 流程、CSP、核心快捷键和 x64 候选包；继续 ARM64、Narrator/Accessibility Insights、中英文文档、WinGet、Store 和供应链验证 |
| Stage 4 | 1.0 | API 冻结、文档完成、三渠道正式发布 |

每个阶段必须有独立工作日志，记录目标、发现、修改、验证、遗留问题和发布结果。
