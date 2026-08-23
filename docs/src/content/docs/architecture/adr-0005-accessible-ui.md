---
title: ADR-0005：可访问桌面 UI 迁移
description: Iced 辅助功能缺口、许可约束和候选迁移路线。
---

## 状态

提议，2026-08-24。Iced 版本保持可发布基线，候选 UI 在功能对等、许可证和双架构验证完成前不替换它。

## 背景

ZiFile 的上架质量要求包含键盘、Narrator/UI Automation、高对比度、DPI 和 IME。当前 Iced 0.14 依赖树没有 AccessKit，Iced 官方的 [accessibility support issue](https://github.com/iced-rs/iced/issues/552) 仍为开放状态。因此仅给画布控件增加文字标签，不能形成可供 Windows 辅助技术消费的语义树。

## 候选

- Slint 默认启用操作系统辅助功能集成，并提供 role、label、live region 和 landmark 属性。但 Slint 运行时采用 GPLv3、Royalty-free 或商业许可；MIT 源码可以保留，完整发行物却不能被描述为纯 MIT 依赖组合，选择 Royalty-free 还需要公开归属。
- Dioxus Desktop 以 Rust RSX 构建 HTML 控件，Windows 使用系统 WebView2；标准 DOM 能沿用浏览器的 UI Automation/屏幕阅读器实现。Dioxus 本身为 MIT/Apache-2.0，但会引入 WebView2 运行时、Web 资源安全策略和新的打包/缓存测试面。
- 直接在 Iced 上手写 AccessKit 适配器需要稳定控件 ID、布局树、命中测试、焦点、文本编辑和动作回调；这实质上是在应用仓库维护 GUI 工具包级功能，不作为产品路线。

## 决策方向

以 Dioxus Desktop + WebView2 建立功能对等候选：保留 Rust 状态与 Worker IPC，UI 使用语义 HTML、键盘焦点顺序、landmark、live region 和原生表单控件。候选必须先通过格式/Worker 功能对等、x64/ARM64 MSIX、离线资源、CSP、无远程导航、Narrator 和 Accessibility Insights，才替换 Iced。

在迁移完成前，文档不得宣称当前桌面已通过屏幕阅读器认证。Slint 保留为技术备选，但只有在许可选择被明确记录并通过依赖策略后才能引入。
