---
title: 桌面使用与辅助功能
description: ZiFile 桌面端的语言、主题、快捷键、大列表行为与待验证边界。
---

ZiFile 桌面端以 Rust 和 Iced 实现。压缩、校验与解压在后台任务中执行，界面显示条目/字节进度，并允许取消；文件名、文件内容和密码不会上传。

## CLI 密码输入

CLI 不接受会进入进程参数和命令历史的 `--password <值>`。加密的 `list`、`test`、`extract` 和 `create` 使用 `--password-stdin`，从标准输入读取一行非空密码；只删除行尾，密码前后空格会保留。

```powershell
$password | zifile test archive.7z --password-stdin
$password | zifile extract archive.7z output --password-stdin
```

## 语言与设置

首次启动根据系统区域选择简体中文或英文。左下角可随时切换语言和深浅主题。ZiFile 只在 `%LOCALAPPDATA%\ZiFile\settings.conf` 保存语言和主题，不保存压缩密码、路径或最近打开记录。

## 键盘快捷键

| 快捷键 | 操作 |
| --- | --- |
| `Ctrl+O` | 打开压缩文件 |
| `Ctrl+N` | 进入创建页 |
| `Ctrl+A` | 在归档页选择全部条目 |
| `Escape` | 取消当前可取消任务 |

## 大型归档

归档路径可即时搜索。结果按每页 500 项显示，避免把整个大型归档一次性构造成 UI 控件。搜索会返回第一页；上一页和下一页按钮用于浏览结果。安全核心仍会在列出阶段执行条目数、展开大小和压缩倍率限制。

## Windows 任务栏

Worker 的字节进度（无字节总量时回退到条目进度）同步到 Windows 任务栏；未知总量使用不确定状态，取消中使用暂停状态，任务结束后清除。任务栏集成失败不得影响归档操作。

## 操作队列

运行归档任务时仍可提交打开、重载、完整性校验、解压或创建。ZiFile 按提交顺序串行执行，状态栏显示等待数量；“清空队列”只删除尚未开始的任务，“取消”只取消当前 Worker，随后继续下一项。队列最多容纳 32 项（包括运行中任务），避免无限占用内存。

每个排队请求只保存在当前进程内，并在提交时快照来源、目标、冲突策略和密码。清空、执行完成或退出会释放这些数据；设置文件和日志不记录队列或密码。对目标路径有先后依赖的任务仍应按安全顺序提交，因为 ZiFile 不会猜测任务间依赖。

## 已验证与待验证

自动测试与 Criterion 已覆盖 100,000 个模拟条目的过滤和有界分页；Windows 实机已检查 1,200 项 ZIP 的三页翻页、搜索、中英文、深浅主题及 `Ctrl+N`。Dioxus 候选另以真实 2 项 ZIP 验证归档区 `Ctrl+A`、动态选择标签与 live status，并以真实 100,000 项 ZIP 验证 Worker 列出、500 行有界 UIA 表格、搜索、多页导航和加载取消。五轮加载取消均进入最终取消状态，且对应 Worker 已退出。

独立键盘回合已从干净启动点用 `Tab`/`Shift+Tab`/`Enter` 正反向切换首页、归档页与创建页，并用键盘切换主题和语言；`Ctrl+N`、`Ctrl+O` 与文件选择器 `Escape` 取消也通过。随后新增的前台保护脚本直接读取 WebView2 内部 `FocusedElement`，在中英文流程中复验导航顺序、disabled 按钮跳过、7z 格式选择、压缩等级 `6→7→6`、密码键入/清空和来源按钮可达。测试不会输出密码，用户切换到其他窗口时拒绝继续发送按键。

归档选择区现在把全选复选框命名为“选择全部归档文件”或“清除全部归档文件选择”，并用原子 `aria-live` 状态报告“已选择 N/总数项”。归档表格区域和“解压选中项”按钮通过 `aria-describedby` 引用同一摘要；单项勾选或取消会在全局状态中报告路径和最新数量。中英文动作、摘要、单复数和状态变化由候选二进制的纯 Rust 单测覆盖。该证据证明语义接线和状态文案，不替代 Narrator 实机遍历。

全局状态区区分普通信息与错误：进度、排队、取消和选择变化使用 `role=status`/polite；Worker 失败、队列满、意外 Worker 结果和内部队列错误使用原子 `role=alert`/assertive，并提供普通主题错误强调与强制颜色系统色。单测锁定这一“只让错误打断”的契约，避免高频进度反复打断屏幕阅读器。

共享队列的 FIFO、容量、陈旧完成保护、清空语义和敏感载荷释放已有纯单元测试，两套 UI 已通过严格 Clippy 与全 feature 测试；真实前台多任务 UI 冒烟仍待无干扰交互回合，因此路线图暂不标记完成。

这些检查尚不等同于完整辅助功能认证。当前 Iced 0.14 UI 不能宣称具备完整的 Windows UI Automation/Narrator 语义树。中文 IME、归档页完整实际键盘/Narrator 遍历、可见焦点、屏幕阅读器、高对比度、每显示器 DPI 和 Windows Application Certification Kit 仍是上架前门禁。可访问 UI 路线见 ADR-0005。

## 可访问 UI 候选

开发者可用以下命令构建候选；它不会替换默认 Iced 可执行文件：

```powershell
cargo build -p zifile-desktop --features accessible-ui --bin zifile-desktop-accessible
target\debug\zifile-desktop-accessible.exe sample.zip
```

候选使用 Rust RSX + Dioxus Desktop/WebView2，并复用 `zifile-worker.exe`。当前可执行打开、列出、校验、选择性解压和创建流程；命令行参数可直接打开归档。x64/ARM64 候选构建、打包、校验和、SBOM、来源证明、离线 CSP、原生拖放和核心快捷键链路已接通；替换默认 UI 前仍需完整辅助功能验证、真实拖放复验和 ARM64 实机运行。
