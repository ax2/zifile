---
title: 测试策略
description: ZiFile 的单元、属性、互操作、安全、性能与冒烟测试要求。
---

## Pull Request 门禁

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features --locked`
- 许可证、来源和安全公告检查
- Starlight 类型检查和静态构建
- Criterion 基准目标可编译
- libFuzzer 目标可编译

## 测试层级

| 类型 | 目标 |
| --- | --- |
| 单元测试 | 格式识别、路径、安全限制、冲突策略 |
| 属性测试 | 随机路径、文件树和边界值 |
| 互操作测试 | 与参考工具双向创建和解压 |
| 安全语料 | Zip Slip、炸弹、链接、冲突、损坏和截断数据 |
| 模糊测试 | 持续攻击格式识别和解析入口 |
| 性能测试 | 吞吐、压缩率、峰值内存、启动和大列表 |
| 冒烟测试 | CLI、桌面启动、安装、升级、文件关联和卸载 |

基础冒烟还会向真实 `zifile-worker.exe` 发送版本化 list 请求，并要求收到 metadata、Unicode entry 和唯一结束事件；随后对 32 MiB 随机输入启动 7z 创建并发送取消控制消息，验证 Worker 在时限内退出、目标不存在且没有临时文件残留。桌面协议单测覆盖缺少终结事件和逐条条目重建；Windows 实机检查验证桌面可通过 Job Object Worker 打开并校验 7z。

任务栏状态映射使用纯单元测试覆盖隐藏、不确定、正常和取消状态；x64 Release/MSIX 构建验证 COM 绑定和 App Execution Alias manifest。任务栏视觉检查与 Narrator/UI Automation 检查仍需在适合的交互式测试环境补证。

性能门禁需要多轮运行和稳定基线，不能用一次共享 CI 结果直接判定回归。

Windows CI 使用 PowerShell `Compress-Archive`/`Expand-Archive` 和系统 `tar.exe`，分别对 ZIP、tar.gz 与 7z 执行双向互操作：参考工具创建的包由 ZiFile 校验和解压，ZiFile 创建的包由参考工具解压并逐文件核对（含 Unicode 路径）。

每次 CI 会编译 libFuzzer 目标；每周定时工作流另外对路径策略和格式识别各运行 180 秒有界 fuzz。失败时保留崩溃产物 14 天。当前目标尚未直接覆盖完整 ZIP/7z 解析器，因此不能把这项门禁描述为解析器已充分模糊测试。

当前 8 MiB 可压缩语料在本地 Windows x64 的一次基线中，ZIP 创建约为 262–275 MiB/s，完整性校验约为 3.04–3.15 GiB/s。该结果只用于建立初始量级，不作为跨机器 CI 阈值。

桌面列表回归测试使用 100,000 个模拟条目，断言搜索结果正确且每次只构造最多 500 个可见行。默认 Iced 与 Dioxus 候选共用 `entry_view` 实现；运行 `cargo bench -p zifile-desktop --bench entry_browser --locked` 可复测。2026-08-24 的 Windows x64 基线中，选择性计数为 16.90–17.62 ms（5.68–5.92 M 条/秒），收集有界页为 15.46–15.96 ms（6.27–6.47 M 条/秒）。Windows 实机还以 100,000 个空条目的真实 ZIP 通过隔离 Worker 打开：UI Automation 报告 100,000 项和 200 页，但表格仅暴露当前 500 行；搜索 `99999` 精确定位末项，搜索 `000` 形成 3 页并可导航到第 3 页。操作后 7 进程当前工作集为 552.70 MiB、私有内存为 313.04 MiB；各进程各自峰值工作集之和为 693.72 MiB，不应解释为同一时刻整树峰值。

`tests/performance/desktop-baseline.ps1` 对优化后的两个桌面程序各启动五次，计时到原生窗口可响应，并在 1.5 秒稳定期后汇总根进程与后代进程内存。参考机器为 Windows 11 build 26200、i9-14900HX：Iced 启动中位数 668.79 ms、工作集 225.87 MiB、私有内存 265.05 MiB；Dioxus/WebView2 候选分别为 294.25 ms、405.91 MiB、206.62 MiB。首轮冷启动包含在 p95 中（Iced 1785.51 ms、候选 644.20 ms）。工作集与私有内存口径不同，该数据是同机回归基线，不代表完整内容就绪、真实十万项归档的精确首屏/滚动延迟或跨机器门禁。
