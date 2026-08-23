---
title: 格式计划
description: ZiFile 计划支持的压缩与归档格式能力矩阵。
---

以下是产品路线图，不代表当前 Stage 0 已经实现压缩和解压。

| 格式 | 浏览 | 解压 | 创建 | 加密 | 计划阶段 |
| --- | --- | --- | --- | --- | --- |
| ZIP | 是 | 是 | 是 | 是 | Alpha |
| 7z | 是 | 是 | 是 | 是 | Beta |
| TAR | 是 | 是 | 是 | 否 | Alpha |
| TAR + gzip/zstd/xz/bzip2/lz4/brotli | 是 | 是 | 是 | 否 | Alpha |
| 单流 gzip/zstd/xz/bzip2/lz4/brotli | 不适用 | 是 | 是 | 否 | Alpha |
| RAR | 是 | 是 | 否 | 待评估 | 1.0 以后 |

RAR 创建不在计划内。RAR 只读能力必须通过许可证和安全评审，且不能改变主项目的 MIT 许可边界。
