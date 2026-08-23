---
title: Stage 0 工作日志
description: ZiFile Foundation 阶段的目标、发现、修改、验证和遗留问题。
---

## 目标

确定产品身份与边界，建立可编译 Rust 工程、Rust UI 技术壳、文档系统、质量门禁和自动发布基础。

## 2026-08-23

### 发现

- `ax2/zifile` GitHub 仓库名和 crates.io `zifile` 名称初步未发现占用。
- 本机具备 Rust 1.88、Node 24、pnpm 10、Git 和已登录的 GitHub CLI。
- Iced 0.14 可满足纯 Rust UI 技术验证，但官方仍标记为实验性，不能跳过 Stage 0 验证。
- Astro Starlight 适合在仓库内维护产品、架构、开发和发布文档。

### 修改

- 创建 Cargo workspace：核心、CLI、桌面三个 crate。
- 建立格式能力注册表、扩展名检测、安全上限及单元测试。
- 创建 Iced 桌面技术壳与 `formats`/`detect` CLI。
- 建立 Starlight 文档站、路线图、ADR、安全和测试文档。
- 添加 CI、Pages、发布、基准和冒烟测试结构。

### 验证

- `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- `cargo test --workspace` 通过：`zifile-core` 5 个单元测试全部成功。
- Foundation 冒烟测试通过：CLI 格式表、复合扩展名识别、桌面 EXE 生成与三秒启动存活检查成功。
- Criterion 首次基线通过：连续识别 5 个常见文件名约 633 ns；该结果只作为本机 Stage 0 基线，不作为跨机器性能承诺。
- `pnpm build` 通过：Astro 类型检查 0 错误、0 警告，Starlight 生成 12 个静态页面及 Pagefind 搜索索引。
- 本机全局 Cargo 的旧 USTC `git://` 索引不可达；本地验证临时改用 HTTPS sparse 镜像，没有修改用户全局设置，也没有把镜像选择提交到仓库。

### 遗留问题

- Iced 大列表、IME、辅助功能、DPI 和高对比度验证。
- Partner Center 名称预留。
- GitHub/WinGet 直接发行物的代码签名方案。
- Stage 1 格式 Provider 的 crate 选择与安全评审。
