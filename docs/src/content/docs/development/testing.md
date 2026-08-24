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
- 发布凭据策略和 MSIX 审计接线冒烟测试

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

共享操作队列单元测试覆盖首项立即启动、后续严格 FIFO、32 项容量边界、满队列不丢失返回载荷、陈旧完成 ID 不推进、清空只影响等待项、等待载荷立即释放，以及 `Debug` 输出不包含载荷。Iced 与 Dioxus 均编译接入同一调度器；在真实前台回合完成“运行中提交至少两项、取消当前项、后续继续、清空等待项”前，不把路线图条目标为完成。

性能门禁需要多轮运行和稳定基线，不能用一次共享 CI 结果直接判定回归。

`tests/reproducibility/windows-build.ps1` 在两个全新目标目录使用固定 Rust 1.88.0、锁文件、单作业构建与 MSVC `/Brepro`，比较默认桌面、可访问候选、CLI、Worker 和 Explorer DLL。2026-08-24 本地 x64 及干净云端 x64/ARM64 证据均为 4/5 匹配；默认 Iced EXE 仍不同，所以门禁按预期失败、路线图未完成。定时/手动双架构工作流用于持续暴露该差异，不得把部分匹配写成整体通过。

Windows CI 使用 PowerShell `Compress-Archive`/`Expand-Archive` 和系统 `tar.exe`，分别对 ZIP、tar.gz 与 7z 执行双向互操作：参考工具创建的包由 ZiFile 校验和解压，ZiFile 创建的包由参考工具解压并逐文件核对（含 Unicode 路径）。

每次 CI 会编译 libFuzzer 目标；每周定时工作流另外对路径策略和格式识别各运行 180 秒有界 fuzz。失败时保留崩溃产物 14 天。当前目标尚未直接覆盖完整 ZIP/7z 解析器，因此不能把这项门禁描述为解析器已充分模糊测试。

当前 8 MiB 可压缩语料在本地 Windows x64 的一次基线中，ZIP 创建约为 262–275 MiB/s，完整性校验约为 3.04–3.15 GiB/s。该结果只用于建立初始量级，不作为跨机器 CI 阈值。

桌面列表回归测试使用 100,000 个模拟条目，断言搜索结果正确且每次只构造最多 500 个可见行。默认 Iced 与 Dioxus 候选共用 `entry_view` 实现；运行 `cargo bench -p zifile-desktop --bench entry_browser --locked` 可复测。2026-08-24 的 Windows x64 基线中，选择性计数为 16.90–17.62 ms（5.68–5.92 M 条/秒），收集有界页为 15.46–15.96 ms（6.27–6.47 M 条/秒）。Windows 实机还以 100,000 个空条目的真实 ZIP 通过隔离 Worker 打开：UI Automation 报告 100,000 项和 200 页，但表格仅暴露当前 500 行；搜索 `99999` 精确定位末项，搜索 `000` 形成 3 页并可导航到第 3 页。

`tests/performance/archive-browser-baseline.ps1` 会确定性生成并在结束后删除 100,000 条目 ZIP，启动匹配的优化版候选和 Worker，使用 UI Automation 等待条目数与页码、滚动到 50%、翻到下一页，并以 25 ms 间隔采样根进程及后代在同一采样时刻的内存。五轮正式基线的窗口启动中位数/p95 为 258.07/308.54 ms，首个归档内容为 3373.80/3668.80 ms，50% 滚动为 195.16/246.34 ms，下一页为 805.87/1143.66 ms；同时刻整树工作集/私有内存最大值为 669.18/455.71 MiB，最多 9 个进程。首内容、滚动和翻页数据包含 UI Automation 观察开销，只用于同机回归，不作为用户可感知延迟的纯应用内测量。

`tests/performance/archive-load-cancellation.ps1` 使用相同的确定性 100,000 条目 ZIP，在候选界面暴露已启用的取消按钮后立即调用，要求最终 live status 为取消错误、不得出现成功打开状态、对应 Worker 必须退出，并在结束时关闭测试实例和删除临时 ZIP。最终五轮基线的取消完成中位数/p95 为 930.78/1088.73 ms；五轮确认时 Worker 数均为 0。该计时从 UI Automation 调用取消开始，到界面收到 Worker 的最终取消结果为止。

`tests/accessibility/keyboard-form.ps1` 从候选原生窗口根开始发送真实 Tab/Shift+Tab/Enter 和表单按键，并通过 UI Automation 读取 WebView2 内部焦点。它验证首页→归档→创建→主题→语言顺序、反向导航、归档/创建页激活、创建页 disabled 按钮不获得焦点、格式选择为 7z、压缩等级 `6→7→6`、密码键入后用 Ctrl+A/Backspace 清空，以及两个来源按钮可达。固定测试密码只在进程内使用，JSON 不记录其值。脚本在每次发送前要求前台原生句柄精确属于本次 ZiFile；用户切换到其他窗口时立即失败并在 `finally` 中关闭测试实例。`-ToggleLanguageBeforeTest` 可用 UIA 建立另一语言前置状态，并在成功或失败清理路径恢复原设置；英文与中文核心流程均已通过。双条目归档页流程的扩展正在实现，只有完成独立真实前台运行后才会加入通过证据。

`tests/smoke/packaging-policy.ps1` 在每次 Windows CI 中解析三个打包脚本，并验证缺失正式凭据、开发 Identity 和未签名 OID Publisher 都会被标签发布门禁拒绝，形式正确的正式输入会被接受；它还要求打包脚本调用 MSIX 审计器，Release workflow 上传 `.audit.json`。真实包审计仍由 `Build-Package.ps1` 在 x64/ARM64 打包后执行，策略冒烟不能替代包内容检查。

`tests/performance/desktop-baseline.ps1` 对优化后的两个桌面程序各启动五次，计时到原生窗口可响应，并在 1.5 秒稳定期后汇总根进程与后代进程内存。参考机器为 Windows 11 build 26200、i9-14900HX：Iced 启动中位数 668.79 ms、工作集 225.87 MiB、私有内存 265.05 MiB；Dioxus/WebView2 候选分别为 294.25 ms、405.91 MiB、206.62 MiB。首轮冷启动包含在 p95 中（Iced 1785.51 ms、候选 644.20 ms）。工作集与私有内存口径不同，该数据是同机回归基线，不代表完整内容就绪、真实十万项归档的精确首屏/滚动延迟或跨机器门禁。
