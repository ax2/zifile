---
title: 安全模型
description: ZiFile 处理不可信压缩包时的威胁、限制和验证要求。
---

## 信任边界

压缩包内容、文件名、元数据、链接、压缩参数和密码提示均不可信。打开列表也属于解析攻击面，并非只有写盘时才需要防护。

## 必须阻止

- `..`、绝对路径、UNC 和目标目录逃逸。
- 符号链接、硬链接、junction 和 reparse point 越界。
- Windows 设备名、NTFS Alternate Data Streams 和非法路径。
- 大小写、Unicode 规范化及重复条目冲突。
- 超过文件数、展开体积、目录深度或压缩比上限的任务。
- 未经确认覆盖现有文件。
- 密码进入命令历史、日志、崩溃报告或进程参数。

## 默认限制

Stage 0 已在 `zifile-core::SafetyLimits` 中建立保守上限。公开的 `list_archive_with_limits` 和 `test_archive_with_limits` 允许调用方在解析列表时收紧限制；`extract_archive` 会把调用方限制传入列出阶段，因此条目数、路径深度、展开量和压缩比会在创建目标目录之前检查。默认便利 API 继续使用统一保守值，写入则使用临时文件和原子替换。

桌面端不在 UI 进程解析归档。版本化 IPC 请求限制为 16 MiB，单个 Worker 事件限制为 4 MiB，归档条目逐条传输。Worker 的 Windows Job Object 限制为一个活动进程和 4 GiB 进程内存，并启用 kill-on-close；创建和解压优先协作取消，2 秒超时后才强制回收。密码只经标准输入发送，不进入命令行。该机制不削减当前用户的文件系统权限，AppContainer 仍属于后续纵深防御。

CLI 不接受明文 `--password` 参数；需要密码的命令只能显式使用 `--password-stdin` 读取一行，避免密码进入进程命令行。调用者仍应从安全提示或秘密提供器写入管道，不应把真实密码作为命令文本字面量。

7z 与 RAR Provider 都在读取、校验和解压入口设置窄范围 unwind 边界。RAR 还会在解码前拒绝 Unix 链接、Windows reparse 条目和 RAR 5+ 重定向，按实际解码字节独立计数，并在整个操作成功前只保留临时文件。若第三方解析器因畸形元数据触发可恢复的 Rust panic，核心返回普通后端错误；OOM、进程终止和 sanitizer 发现不会被该边界截获，仍由 Worker 隔离和 fuzz 门禁处理。

## 依赖与许可证

默认发行物只允许经过批准的宽松许可证。`cargo-deny` 阻止未知 registry、未知 Git 来源和通配依赖。RAR Provider 使用 MIT OR Apache-2.0 的 `rars`，不会改变 ZiFile 的 MIT 分发边界；升级仍必须重新执行依赖、语料、fuzz 与互操作评审。
