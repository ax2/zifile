---
title: 公开契约与版本策略
description: ZiFile 1.0 的 CLI、核心 Provider、IPC 与版本兼容边界。
---

本页定义 1.0 候选契约。正式发布 1.0 前仍可在更新日志和迁移说明中调整；1.0 后遵循语义化版本，破坏性变更只进入新的主版本。

## CLI 契约

1.0 候选保留以下命令：`formats`、`detect`、`list`、`test`、`extract`、`create`。创建格式值为 `zip`、`seven-zip`、`tar`、`tar-gzip`、`tar-zstd`、`tar-xz`、`tar-bzip2`、`gzip`、`zstandard`、`xz`、`bzip2`、`lz4` 和 `brotli`；冲突策略为 `overwrite`、`skip`、`rename` 和 `error`。

密码输入只允许显式 `--password-stdin`。CLI 不接受明文密码参数，也不保证交互式提示。

| 退出码 | 含义 |
| --- | --- |
| `0` | 操作成功 |
| `1` | 文件、格式、密码、策略、后端或其他运行时错误 |
| `2` | Clap 检测到命令行语法或参数错误 |

运行时错误写入标准错误并使用 `error: ` 前缀。普通成功文案面向用户阅读，可在不改变命令语义的情况下改进；自动化不应解析这些自然语言句子。`zifile formats` 是稳定的制表符分隔能力表，包含 `CREATE_INPUT` 列：`files-or-directories`、`single-file` 或 `none`。

## 核心 Provider 契约

桌面、CLI 和 Worker 共用 `zifile-core`。1.0 候选边界包含：

- `ArchiveFormat`、`FormatCapabilities`、`CreateInputKind` 和 `ReleaseStage`；
- 检测、列出、校验、创建和解压入口；
- `CreateOptions`、`ExtractOptions`、`ConflictPolicy`、`SafetyLimits` 与取消/进度类型；
- `ZiFileError` 与 `ZiFileResult`。

增加新格式、能力或非必填选项属于兼容扩展。删除或重命名公开格式、改变既有选项默认安全语义、放宽安全限制，或重新解释已有错误，属于需要主版本评审的变更。RAR 创建不属于 1.0 契约。

Worker JSON Lines IPC 使用独立的 `PROTOCOL_VERSION`。不兼容客户端/Worker 必须明确拒绝，不能根据字段猜测版本。

## 单一版本来源

`Cargo.toml` 的 `[workspace.package].version` 是产品版本唯一来源。文档包、六个工作区包、内部依赖 pin 和 `Cargo.lock` 必须一致。发布标签必须精确为 `v<workspace-version>`；MSIX 四段版本由同一值确定性转换，例如 `0.1.0-alpha.1` 转为 `0.1.0.1`。

`scripts/Test-VersionConsistency.ps1` 在普通 CI 和 Release 工作流中执行。手动 Release 验证不再接收可变版本输入，始终构建当前工作区版本。
