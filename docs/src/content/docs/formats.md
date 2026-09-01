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
| TAR + gzip/zstd/xz/LZMA/bzip2/LZ4 | 是 | 是 | 是 | 否 | 已实现 |
| 单流 gzip/zstd/xz/lzma/bzip2/lz4/brotli | 单条目 | 是 | 是 | 否 | 已实现 |
| RAR 1.3–7 | 是 | 是 | 否 | 读取 | Beta |
| Windows CAB | 是 | 是 | 是（MSZIP） | 否 | Beta |

ZIP 读取支持 Store、Deflate、Deflate64、BZip2、LZMA、XZ、Zstandard 与 PPMd 方法，也可解密 AES 和传统 ZipCrypto 归档。创建使用兼容性良好的 Deflate，密码创建使用 AES-256；传统 ZipCrypto 仅用于读取旧归档，不作为新的加密选项。Store、Deflate、Deflate64、BZip2、LZMA、XZ、PPMd、AES-256 与 ZipCrypto 会在 Windows CI 中使用独立 7-Zip 参考语料验证；Zstandard 解码使用固定的 libarchive ZIPX 语料独立验证，并精确核对路径、大小与逐文件哈希。

`.zipx` 会作为 ZIP 读取别名识别，并已加入两套桌面打开对话框和 Windows 安装包文件关联；ZiFile 默认仍创建普通 `.zip` 归档。

桌面打开对话框也会显示常见漫画、TAR 家族、LZMA、Bzip2 与 LZ4 别名，例如 CBZ/CB7/CBR/CBT、TXZ/TZST/TBZ2、`.tar.lzma`、`.tlz4`、`.lzma` 和 `.bz`。其中 `.tar.lzma` 使用 TAR + LZMA-alone，`.tar.lz4`/`.tlz4` 使用 TAR + LZ4，普通 `.lzma` 使用 LZMA-alone 解码器，`.lz4` 使用单流 LZ4 解码器；Windows 安装包注册单段归档后缀，但 Appx 清单不接受复合后缀，因此复合格式可在 ZiFile 打开对话框中选择，却不会被 MSIX 默认文件关联接管；`.epub` 也不会默认接管，仍可在 ZiFile 中手动选择并作为 ZIP 内容检查。

历史 ZIP 的 Shrink、Reduce 1–4 与 Implode 方法也支持只读解码。固定上游语料用于校验 ZiFile 解压后的字节与已知内容完全一致，7-Zip 则独立识别归档所用方法；这些过时算法不会作为新归档的创建选项。

创建 ZIP、7z、CAB 和 TAR 组合（包括 TAR + LZMA 与 TAR + LZ4）时可以选择多个文件或文件夹。CAB 创建使用固定的 MSZIP 编码，不显示无效的压缩等级；空目录无法由 CAB 容器表达，选择包含空目录的来源会明确拒绝。单流 gzip、Zstandard、XZ、LZMA、Bzip2、LZ4 和 Brotli 必须恰好选择一个现有文件；若要压缩目录或多个项目，请选择对应的 TAR 组合格式。桌面端会在打开保存对话框前检查这一要求。

解压单流格式时，`.lzma` 和 `.bz` 等历史别名会沿用原始文件名主体，不会因为格式的规范后缀不同而额外生成 `.out`。

创建界面的压缩等级范围由所选编码器决定：ZIP、7z、gzip、XZ 和 LZMA 为 0–9，Zstandard 为 0–22，Bzip2 为 1–9，Brotli 为 0–11。纯 TAR、CAB（固定 MSZIP）和当前 LZ4 编码器使用固定设置，因此这些格式不会显示无效的等级滑块。CLI 的 `zifile formats` 会公开相同范围；`create --level` 会拒绝越界值，TAR/CAB/LZ4 显式传入等级也会被拒绝，避免静默钳制或忽略。7z 会把所选等级写入 LZMA2 参数，加密时仍沿用相同等级而不是退回后端默认值。

RAR 创建不在计划内。只读浏览、完整性测试和选择性解压使用纯 Rust 的 `rars` Provider（MIT OR Apache-2.0）。ZiFile 会拒绝不安全路径、链接和 RAR 5+ 重定向，执行声明大小与实际解码大小限制，先写临时文件，并在隔离 Worker 中处理归档。密码保护的 RAR 可以读取，密码不会持久化。

CAB 使用纯 Rust、MIT 许可的 `cab` Provider。创建使用固定 MSZIP 编码，并支持浏览、完整性校验和选择性安全解压 None、MSZIP 与 LZX 内容；Quantum 压缩和跨多个 Cabinet 的集合暂不支持，多 Cabinet 头部会在浏览前被明确拒绝。CAB 创建不支持密码，也不能安全更新或重命名已存在的容器。
