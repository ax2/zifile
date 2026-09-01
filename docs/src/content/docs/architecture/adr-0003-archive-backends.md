---
title: ADR-0003：归档后端组合
description: 记录 ZiFile 当前格式实现、许可证边界与替换策略。
---

- 状态：接受
- 日期：2026-08-24

## 决定

核心采用纯 Rust 或 Rust 原生绑定的可替换后端组合：ZIP 使用 `zip`，7z 使用 `sevenz-rust2`，RAR 读取与 RAR 5 创建使用 `rars`，CAB 创建与读取使用 `cab`，TAR 使用 `tar`，gzip 使用 `flate2`，Zstandard 使用 `zstd`，XZ 使用静态链接的 `xz2`，standalone LZMA 使用 `lzma-rust2`，Bzip2 使用 `bzip2`，LZ4 使用 `lz4_flex`，Brotli 使用 `brotli`。

所有后端只能通过 `zifile-core` 的统一入口暴露能力。路径规范化、链接拒绝、冲突处理、资源上限、取消和临时文件写入由核心统一实施，不能交给 UI 或调用者自行拼装。

## 约束

- 每次新增或替换后端都要重新运行许可证、来源、安全语料、互操作、fuzz 与性能检查。
- 能力矩阵只报告真实实现并有测试证据的操作。
- RAR 1.3–7 读取和 RAR 5 创建使用通过许可证和来源评审的 `rars` 0.9.3（MIT OR Apache-2.0）；创建支持等级 0–5、密码加密头和原子输出，但分卷、恢复记录、更新和重命名仍不支持。Beta 能力继续受核心安全检查、Worker 隔离、fuzz、恶意夹具和参考读取器互操作门禁约束。
- CAB 新建使用 MIT 许可的 `cab` 0.6.0，以固定 MSZIP 编码写出且不支持密码；None、MSZIP 和 LZX 解码进入 Beta，Quantum 与跨 Cabinet 集合明确不支持。CAB 的固定容器布局不提供更新或重命名。
- C/C++ 后端只在 Rust 方案无法满足兼容性且完成单独供应链评审时采用。

## 结果

当前组合覆盖 Windows 日常使用的主要开放格式，同时维持 MIT 应用的清晰分发边界。代价是各格式元数据能力并不完全一致，例如 7z 条目级加密标记需要后端提供可靠信息后才能显示。
