---
title: 贡献指南
description: ZiFile 的开发环境、变更要求、测试证据与 Pull Request 规则。
---

## 开始之前

ZiFile 的归档输入属于不可信数据，Windows 桌面在辅助功能迁移期间同时维护 Iced 基线和 Dioxus/WebView2 候选，打包与发布还有独立门禁。大型格式、安全、UI、IPC 或发行变更应先建立设计 Issue；小型修复可直接提交聚焦的 Pull Request。

Windows 10/11 是主要产品环境。Rust 版本由 `rust-toolchain.toml` 固定，Rust 与文档依赖分别使用仓库中的 `Cargo.lock` 和 `docs/pnpm-lock.yaml`，不得无说明地改用其他工具链或未锁定依赖。

## 基础门禁

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features --locked
./tests/smoke/foundation.ps1 -SkipDesktopLaunch
./tests/smoke/packaging-policy.ps1
pnpm --dir docs install --frozen-lockfile
pnpm --dir docs build
```

还需运行与修改最相关的互操作、性能、辅助功能或打包脚本。真实窗口脚本只能在可用的交互桌面运行，必须保留前台窗口保护；不能用编译或静态 UIA 检查替代真实 Narrator、IME、DPI、高对比度或跨窗口拖放结论。

## 变更要求

- 归档 Provider：更新能力注册表，并提供往返或只读语料、资源限制、恶意输入、取消及无临时输出证据。
- 桌面行为：默认 UI 决策前，共享流程必须同时检查 Iced 基线与 Dioxus 候选；可访问语义变化还需覆盖键盘、焦点、名称、状态与播报边界。
- 文案与文档：用户可见内容同步简体中文和英文；Starlight 页面按相同路径成对维护。
- 公开接口：CLI、核心 Provider 与 IPC 遵循[公开契约与版本策略](/zifile/development/contracts/)。
- 发布材料：显著功能或流程变化更新 `CHANGELOG.md`；只有准备对应标签时才切出带日期的版本章节。
- 架构：新决策在 `docs/src/content/docs/architecture/` 增加 ADR。

## 安全与证据

不要提交密码、令牌、Cookie、私钥、签名文件、客户归档或真实敏感数据；漏洞按照根目录 `SECURITY.md` 私下报告。提交说明必须区分“已实现”“本地验证”“云端验证”和“仍需实体设备/账号/认证”；未签名包、静态 manifest 或 readiness 结果不能写成可信安装、WACK、Store 或 WinGet 已通过。

只暂存本次修改的明确文件，保留其他人的工作区改动。Windows 阶段产物保存完整可运行目录和 EXE，不保留 ZIP，除非任务明确要求 ZIP。
