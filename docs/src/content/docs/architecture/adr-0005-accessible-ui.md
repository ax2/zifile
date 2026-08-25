---
title: ADR-0005：可访问桌面 UI 迁移
description: Iced 辅助功能缺口、许可约束和候选迁移路线。
---

## 状态

接受并进入验证，2026-08-24。Iced 版本保持可发布基线，候选 UI 在功能对等、许可证和双架构验证完成前不替换它。

## 背景

ZiFile 的上架质量要求包含键盘、Narrator/UI Automation、高对比度、DPI 和 IME。当前 Iced 0.14 依赖树没有 AccessKit，Iced 官方的 [accessibility support issue](https://github.com/iced-rs/iced/issues/552) 仍为开放状态。因此仅给画布控件增加文字标签，不能形成可供 Windows 辅助技术消费的语义树。

## 候选

- Slint 默认启用操作系统辅助功能集成，并提供 role、label、live region 和 landmark 属性。但 Slint 运行时采用 GPLv3、Royalty-free 或商业许可；MIT 源码可以保留，完整发行物却不能被描述为纯 MIT 依赖组合，选择 Royalty-free 还需要公开归属。
- Dioxus Desktop 以 Rust RSX 构建 HTML 控件，Windows 使用系统 WebView2；标准 DOM 能沿用浏览器的 UI Automation/屏幕阅读器实现。Dioxus 本身为 MIT/Apache-2.0，但会引入 WebView2 运行时、Web 资源安全策略和新的打包/缓存测试面。
- 直接在 Iced 上手写 AccessKit 适配器需要稳定控件 ID、布局树、命中测试、焦点、文本编辑和动作回调；这实质上是在应用仓库维护 GUI 工具包级功能，不作为产品路线。

## 决策方向

以 Dioxus Desktop + WebView2 建立功能对等候选：保留 Rust 状态与 Worker IPC，UI 使用语义 HTML、键盘焦点顺序、landmark、live region 和原生表单控件。候选必须先通过格式/Worker 功能对等、x64/ARM64 MSIX、离线资源、CSP、无远程导航、Narrator 和 Accessibility Insights，才替换 Iced。

在迁移完成前，文档不得宣称当前桌面已通过屏幕阅读器认证。Slint 保留为技术备选，但只有在许可选择被明确记录并通过依赖策略后才能引入。

## 当前证据

仓库已加入需显式启用 `accessible-ui` feature 的 `zifile-desktop-accessible` 候选。它复用设置、任务栏、版本化 Worker IPC 和核心安全限制，包含首页、命令行打开、归档列表/筛选/分页/选择、完整性校验、解压配置、创建来源/格式/压缩等级/密码、进度与取消。

Windows 实机 UI Automation 已识别主导航、标题层级、归档表格、复选框、组合框、滑块、密码框和 live status。真实 ZIP 经命令行打开后列出 2 个项目并完成完整性校验。候选现已接入原生文件拖放、`Ctrl+O`、`Ctrl+N`、条目区限定的 `Ctrl+A` 与 `Escape`，并用 CSP 将脚本、样式、图像和连接限制到内联 UI、本地 Dioxus 协议及回环 WebSocket；实机确认收紧后界面和语义树仍可运行。归档全选控件现在区分“选择全部/清除全部”动作，原子 live summary 报告选择数，归档区域与解压按钮引用同一摘要，单项变化报告路径与数量；创建来源列表同样提供 live 数量，每个移除按钮包含目标路径，并按路径而非过期索引移除。中英文语义文案由 Rust 单测覆盖。全局播报器仅对 Worker/队列错误使用原子 alert/assertive，普通进度与选择信息保持 status/polite，并用普通/强制颜色样式区分失败。动态选择标签在真实窗口中从 0 项更新到 2 项。独立键盘回合已证明主导航正反向遍历、主题/语言切换及 Ctrl+O/Ctrl+N/文件对话框 Escape；后续脚本直接读取 WebView2 内部焦点，并在中英文状态下验证创建表单格式、滑块、密码、来源按钮与 disabled 跳过。脚本以精确前台窗口句柄防止把按键发送到用户其他应用。确定性十万条目加载取消五轮均得到最终取消状态，对应 Worker 在确认时全部退出。Release 演练 32667737142 已为 x64/ARM64 构建、打包、证明并上传候选 EXE 与 MSIX，下载复核的 12 个校验目标全部匹配，PE 架构标识正确且无 ZIP 发布物。该证据仍不等同于 Narrator 或可见焦点认证，也没有证明高对比度实机视觉、中文 IME、每显示器 DPI、真实跨窗口拖放或 ARM64 实机运行。
