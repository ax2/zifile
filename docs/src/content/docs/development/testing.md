---
title: 测试策略
description: ZiFile 的单元、属性、互操作、安全、性能与冒烟测试要求。
---

## Pull Request 门禁

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
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

性能门禁需要多轮运行和稳定基线，不能用一次共享 CI 结果直接判定回归。

Windows CI 使用 PowerShell `Compress-Archive`/`Expand-Archive` 和系统 `tar.exe`，分别对 ZIP、tar.gz 与 7z 执行双向互操作：参考工具创建的包由 ZiFile 校验和解压，ZiFile 创建的包由参考工具解压并逐文件核对（含 Unicode 路径）。

每次 CI 会编译 libFuzzer 目标；每周定时工作流另外对路径策略和格式识别各运行 180 秒有界 fuzz。失败时保留崩溃产物 14 天。当前目标尚未直接覆盖完整 ZIP/7z 解析器，因此不能把这项门禁描述为解析器已充分模糊测试。

当前 8 MiB 可压缩语料在本地 Windows x64 的一次基线中，ZIP 创建约为 262–275 MiB/s，完整性校验约为 3.04–3.15 GiB/s。该结果只用于建立初始量级，不作为跨机器 CI 阈值。

桌面列表回归测试使用 100,000 个模拟条目，断言搜索结果正确且每次只构造最多 500 个可见行。2026-08-24 的 Windows 实机检查另用 1,200 项 ZIP 验证三页翻页和单项搜索。此结果证明渲染结构有界，但不替代十万条目峰值内存、首屏延迟和滚动响应的正式基准。
