---
title: 格式支持
description: ZiFile 当前已验证和计划中的压缩与归档格式能力矩阵。
---

以下能力已由仓库内的集成测试覆盖；“计划”项目不会在 UI 中宣称可用。

| 格式 | 浏览 | 解压 | 创建 | 加密 | 状态 |
| --- | --- | --- | --- | --- | --- |
| ZIP | 是 | 是 | 是 | AES | 已实现 |
| 7z | 是 | 是 | 是 | AES | 已实现 |
| TAR | 是 | 是 | 是 | 否 | 已实现 |
| TAR + gzip/zstd/xz/bzip2 | 是 | 是 | 是 | 否 | 已实现 |
| 单流 gzip/zstd/xz/bzip2/lz4/brotli | 单条目 | 是 | 是 | 否 | 已实现 |
| RAR 1.3–7 | 是 | 是 | 否 | 读取 | Beta |
| Windows CAB | 是 | 是 | 否 | 否 | Beta |

ZIP 读取支持 Store、Deflate、Deflate64、BZip2、LZMA、XZ、Zstandard 与 PPMd 方法，也可解密 AES 和传统 ZipCrypto 归档。创建使用兼容性良好的 Deflate，密码创建使用 AES-256；传统 ZipCrypto 仅用于读取旧归档，不作为新的加密选项。Store、Deflate、Deflate64、BZip2、LZMA、XZ、PPMd、AES-256 与 ZipCrypto 会在 Windows CI 中使用独立 7-Zip 参考语料验证；Zstandard 的 ZIP 解码暂由锁定后端测试覆盖，不宣称已有 7-Zip 语料证明。

历史 ZIP 的 Shrink、Reduce 1–4 与 Implode 方法也支持只读解码。固定上游语料用于校验 ZiFile 解压后的字节与已知内容完全一致，7-Zip 则独立识别归档所用方法；这些过时算法不会作为新归档的创建选项。

创建 ZIP、7z 和 TAR 组合时可以选择多个文件或文件夹。单流 gzip、Zstandard、XZ、Bzip2、LZ4 和 Brotli 必须恰好选择一个现有文件；若要压缩目录或多个项目，请选择对应的 TAR 组合格式。桌面端会在打开保存对话框前检查这一要求。

RAR 创建不在计划内。只读浏览、完整性测试和选择性解压使用纯 Rust 的 `rars` Provider（MIT OR Apache-2.0）。ZiFile 会拒绝不安全路径、链接和 RAR 5+ 重定向，执行声明大小与实际解码大小限制，先写临时文件，并在隔离 Worker 中处理归档。密码保护的 RAR 可以读取，密码不会持久化。

CAB 使用纯 Rust、MIT 许可的 `cab` Provider。当前支持浏览、完整性校验和选择性安全解压 None、MSZIP 与 LZX 内容；Quantum 压缩和跨多个 Cabinet 的集合暂不支持，多 Cabinet 头部会在浏览前被明确拒绝，CAB 创建也不对外开放。
