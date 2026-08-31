---
title: 快速上手
description: 使用 ZiFile 浏览、校验、解压和创建压缩文件。
---

ZiFile 目前是 Stage 4 公开版本，尚未发布 Microsoft Store 或 WinGet 正式版本。GitHub 用户可从当前的 [v0.1.8 Release](https://github.com/ax2/zifile/releases/tag/v0.1.8) 获取未签名 Windows 构建，并应先按 Release 中的 `SHA256SUMS.txt` 校验下载文件。需要安装时只需选择包含 x64 与 ARM64 的一体化 `ZiFile-0.1.8.0-windows.msixbundle`；便携版文件为 `zifile-windows-x64.exe` 和 `zifile-windows-arm64.exe`，均为可独立运行的自包含程序，不需要额外的 Worker 或 DLL。不要为了安装开发包而导入未知根证书或关闭 Windows 安全检查。

## 打开和检查归档

1. 启动 ZiFile，选择“打开归档”，或按 `Ctrl+O`。
2. 选择 ZIP、7z、RAR、CAB、TAR 或受支持的压缩流。也可以把已知归档拖入窗口。
3. 使用带有持久“搜索”标签的输入框过滤路径；输入内容后标签不会消失。大型归档每页显示 500 项。
4. 解压前可先运行“完整性校验”。如果 7z 或 RAR 连文件列表也已加密，首次打开失败后会保留所选文件并显示密码重试界面；密码不会写入设置，打开其他归档时也会清空。

格式由文件签名和扩展名共同识别。扩展名不等于真实内容时，ZiFile 会报告检测或解析错误，不会强行按扩展名解码。

归档标题同时显示展开大小、压缩后大小和缩小比例；如果归档封装开销使文件变大，则明确显示“增大”比例，空归档不显示无意义的百分比。

## 安全解压

选择全部或部分条目，在明确标注的“文件冲突策略”中选择重命名、覆盖、跳过或报错，指定目标文件夹，然后开始解压。ZiFile 默认拒绝路径穿越、绝对路径、Windows 设备名、大小写冲突、不安全链接、包含符号链接/junction/reparse point 的目标路径以及超过条目数、展开大小或压缩倍率限制的内容。遇到这类拒绝时，应核对归档来源，不要通过关闭安全边界重试。

任务在隔离 Worker 中运行。按 `Escape` 或选择“取消”会先请求协作取消，必要时终止 Worker；已经完整写入的文件不会被误报为事务式回滚。

## 创建归档

1. 选择“创建归档”或按 `Ctrl+N`。
2. 添加文件或文件夹，也可以把来源拖入窗口。
3. 选择格式、压缩等级和可选密码，再选择保存位置。

ZIP、7z 和 TAR 组合支持多个文件与文件夹。gzip、Zstandard、XZ、LZMA、Bzip2、LZ4 和 Brotli 是单文件流，必须恰好选择一个现有文件；需要压缩目录时，请改用对应 TAR 组合。RAR 与 CAB 只支持读取，不支持创建。

## 队列与设置

任务运行时仍可提交打开、校验、解压或创建，ZiFile 最多保存 32 项并按顺序执行。“清空队列”只移除等待项，“取消”只作用于当前任务。语言与深浅主题保存在 `%LOCALAPPDATA%\ZiFile\settings.conf`；路径、最近记录和密码不会保存。

“关于”页显示当前运行版本、MIT 许可证、格式系列数量、项目地址和本地处理隐私边界，可按 `F1` 直接打开。页面还可用默认浏览器打开项目主页、中文使用文档和中文隐私政策；若系统拒绝启动链接，底部状态区会显示错误。报告问题时请从这里核对版本，不要仅依赖安装文件名。

## 命令行

```powershell
zifile formats
zifile list archive.zip
zifile test archive.7z
zifile extract archive.zip output --conflict rename
zifile create output.7z files --format seven-zip --level 9
```

`zifile formats` 的 `COMPRESSION_LEVEL` 列会列出每种格式允许的闭区间；`fixed` 表示编码器没有可调等级，必须省略 `--level`。可调格式省略该参数时默认使用等级 6；越界时会明确报错，不会用另一个等级继续创建。

加密操作只接受标准输入，不接受会进入进程参数和普通命令历史的明文密码参数：

```powershell
$password | zifile test archive.7z --password-stdin
$password | zifile extract archive.7z output --password-stdin
```

遇到问题请参阅[故障排查](/zifile/guides/troubleshooting/)；支持矩阵见[格式支持](/zifile/formats/)。
