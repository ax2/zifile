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

CLI 密码单测覆盖显式 opt-in、CRLF/LF 删除、前后空格保留和缺失/空输入拒绝。基础冒烟还要求 CLI 帮助仅暴露 `--password-stdin`，并通过标准输入真实创建、校验和解压 AES 7z；固定测试密码不输出到结果。

任务栏状态映射使用纯单元测试覆盖隐藏、不确定、正常和取消状态；x64 Release/MSIX 构建验证 COM 绑定和 App Execution Alias manifest。任务栏视觉检查与 Narrator/UI Automation 检查仍需在适合的交互式测试环境补证。

共享操作队列单元测试覆盖首项立即启动、后续严格 FIFO、32 项容量边界、满队列不丢失返回载荷、陈旧完成 ID 不推进、清空只影响等待项、等待载荷立即释放，以及 `Debug` 输出不包含载荷。Iced 与 Dioxus 均编译接入同一调度器；在真实前台回合完成“运行中提交至少两项、取消当前项、后续继续、清空等待项”前，不把路线图条目标为完成。

性能门禁需要多轮运行和稳定基线，不能用一次共享 CI 结果直接判定回归。

`tests/reproducibility/windows-build.ps1` 在两个全新目标目录使用固定 Rust 1.93.0、锁文件、单作业构建与 MSVC `/Brepro`，比较默认桌面、可访问候选、CLI、Worker 和 Explorer DLL。云端 32813453959 在新工具链上得到双架构各 4/5；schema v2 运行 32822543635 进一步确认默认 Iced EXE 的 `.rdata` 首差异是 `glutin_wgl_sys` 生成绑定内嵌的 `build-a`/`build-b` target 路径。脚本现用 `CARGO_ENCODED_RUSTFLAGS` 与 `--remap-path-prefix` 把两个根映射到同一虚拟路径；修复后 32826187552 的 x64/ARM64 都达到 5/5 且 `reproducible=true`。

Windows CI 使用 PowerShell `Compress-Archive`/`Expand-Archive` 和系统 `tar.exe`，分别对 ZIP、tar.gz 与 7z 执行双向互操作：参考工具创建的包由 ZiFile 校验和解压，ZiFile 创建的包由参考工具解压并逐文件核对（含 Unicode 路径）。

新增的官方 7-Zip 语料门禁使用 GitHub Windows Runner 上的 `7z.exe`，覆盖 Copy、LZMA、LZMA2+BCJ、Deflate、BZip2、PPMd，以及带文件名加密的 LZMA2+AES；反向还要求官方 7-Zip 校验并解压 ZiFile 创建的普通与 AES 归档。所有场景逐文件比较 SHA-256，并上传不含密码的 JSON 证据。CI 32836336921 使用 7-Zip 26.02 完成 9/9 场景，证据 JSON SHA-256 为 `06278BB8B96AB683A3C117BA5E30F1B4AB1CF89F1BBF01E72BAC0CC26B49DB14`。

RAR 门禁从固定的 `rars` 源码提交 `7d8f9386ef777a2415da34fe1db193d8471ff7d0` 下载六个夹具，使用硬编码 SHA-256 验证来源后，逐文件比较 ZiFile 与 7-Zip 的解压树。覆盖 RAR 1.3、1.54 多文件、RAR 3 PPMd、RAR 5 压缩与 E8E9 过滤，以及 WinRAR 7.21 加密头/Quick Open；另有三个链接/重定向夹具必须在无输出的情况下拒绝。CI 32853686537 完成全部六个有效场景和三个拒绝场景，证据 JSON SHA-256 为 `4C52D0240B911609C7DDB0CACB2E484F56C8F886E216347603B228261C4EE8EF`。RAR 1.3 因现代 7-Zip 不再读取，改与同一固定上游提交中的已知正确解压树逐文件核对，其余五种有效归档继续与 7-Zip 26.02 交叉验证。

每次 CI 会编译 libFuzzer 目标；每周定时工作流对路径策略、格式识别和归档解析器各运行 180 秒有界 fuzz，失败时保留崩溃产物 14 天。归档目标用带格式签名的变异输入覆盖 ZIP、7z、RAR、四种 TAR 组合和六种单流格式，限制输入为 256 KiB、RSS 为 2 GiB、单输入为 10 秒；目标内部还使用 256 条目、4 MiB 展开量、64 倍压缩比和 32 层路径的严格限制。Windows/MSVC 本地 `cargo-fuzz` 因 `sevenz-rust2` DLL 与 libFuzzer 入口点参数冲突无法链接，Rust 编译检查可通过；动态 campaign 以 Linux GNU 定时工作流为验收环境。损坏、截断、炸弹及更多 libarchive 变体仍需继续扩展。

首轮归档解析器 campaign 32733658052 在约 569,000 次执行后发现 292 字节输入可令 `sevenz-rust2` 0.20.2 的文件数量分配触发 `capacity overflow`。第二轮 campaign 32803785688 又发现 173 字节输入可触发 ASan 超大内存分配；这类 OOM 不能由 `catch_unwind` 可靠隔离。两份输入均已转为永久集成测试和 fuzz 启动重放夹具。项目因此升级到 Rust 1.93.0 与带有元数据计数边界修复的 `sevenz-rust2` 0.22.0；Provider 的 panic 边界继续作为纵深防御。升级后的定向 campaign 32813469578 强制重放两份样本，并在 181 秒内继续执行 498,937 次、峰值 RSS 370 MiB，未产生新崩溃产物。

`libfuzzer-sys` 默认 panic hook 会在 unwind 前 abort，绕过 Provider 的错误边界。归档 fuzz 目标只在自身进程初始化时移除这个 hook；libFuzzer 外层仍会令任何逃出 ZiFile 的 panic 失败。目标启动时必定重放上述固定样本，防止新 campaign 因随机语料未再次命中而产生假通过。

当前 8 MiB 可压缩语料在本地 Windows x64 的一次基线中，ZIP 创建约为 262–275 MiB/s，完整性校验约为 3.04–3.15 GiB/s。该结果只用于建立初始量级，不作为跨机器 CI 阈值。

RAR 校验基准使用确定性的 8 MiB RAR 5 method-3 归档，并加入低频伪随机噪声，在保留压缩工作的同时不超过默认 `1000:1` 膨胀保护。首次本地 Windows x64 基线为 58.12–64.49 ms，即 124.06–137.65 MiB/s；它只用于同机回归，不是通用性能承诺。最初的高度周期性夹具被安全比率按设计拒绝，没有为了跑分绕过该保护。

桌面列表回归测试使用 100,000 个模拟条目，断言搜索结果正确且每次只构造最多 500 个可见行。默认 Iced 与 Dioxus 候选共用 `entry_view` 实现；运行 `cargo bench -p zifile-desktop --bench entry_browser --locked` 可复测。2026-08-24 的 Windows x64 基线中，选择性计数为 16.90–17.62 ms（5.68–5.92 M 条/秒），收集有界页为 15.46–15.96 ms（6.27–6.47 M 条/秒）。Windows 实机还以 100,000 个空条目的真实 ZIP 通过隔离 Worker 打开：UI Automation 报告 100,000 项和 200 页，但表格仅暴露当前 500 行；搜索 `99999` 精确定位末项，搜索 `000` 形成 3 页并可导航到第 3 页。

`tests/performance/archive-browser-baseline.ps1` 会确定性生成并在结束后删除 100,000 条目 ZIP，启动匹配的优化版候选和 Worker，使用 UI Automation 等待条目数与页码、滚动到 50%、翻到下一页，并以 25 ms 间隔采样根进程及后代在同一采样时刻的内存。五轮正式基线的窗口启动中位数/p95 为 258.07/308.54 ms，首个归档内容为 3373.80/3668.80 ms，50% 滚动为 195.16/246.34 ms，下一页为 805.87/1143.66 ms；同时刻整树工作集/私有内存最大值为 669.18/455.71 MiB，最多 9 个进程。首内容、滚动和翻页数据包含 UI Automation 观察开销，只用于同机回归，不作为用户可感知延迟的纯应用内测量。

`tests/performance/archive-load-cancellation.ps1` 使用相同的确定性 100,000 条目 ZIP，在候选界面暴露已启用的取消按钮后立即调用，要求最终 live status 为取消错误、不得出现成功打开状态、对应 Worker 必须退出，并在结束时关闭测试实例和删除临时 ZIP。最终五轮基线的取消完成中位数/p95 为 930.78/1088.73 ms；五轮确认时 Worker 数均为 0。该计时从 UI Automation 调用取消开始，到界面收到 Worker 的最终取消结果为止。

`tests/accessibility/keyboard-form.ps1` 从候选原生窗口根开始发送真实 Tab/Shift+Tab/Enter 和表单按键，并通过 UI Automation 读取 WebView2 内部焦点。它验证首页→归档→创建→主题→语言顺序、反向导航、归档/创建页激活、创建页 disabled 按钮不获得焦点、格式选择为 7z、压缩等级 `6→7→6`、密码键入后用 Ctrl+A/Backspace 清空，以及两个来源按钮可达。固定测试密码只在进程内使用，JSON 不记录其值。脚本在每次发送前要求前台原生句柄精确属于本次 ZiFile；用户切换到其他窗口时立即失败并在 `finally` 中关闭测试实例。`-ToggleLanguageBeforeTest` 可用 UIA 建立另一语言前置状态，并在成功或失败清理路径恢复原设置；英文与中文核心流程均已通过。双条目归档页流程的扩展正在实现，只有完成独立真实前台运行后才会加入通过证据。

`tests/smoke/packaging-policy.ps1` 在每次 Windows CI 中解析三个打包脚本，并验证缺失正式凭据、开发 Identity 和未签名 OID Publisher 都会被标签发布门禁拒绝，形式正确的正式输入会被接受；它还要求打包脚本调用 MSIX 审计器，Release workflow 上传 `.audit.json`。真实包审计仍由 `Build-Package.ps1` 在 x64/ARM64 打包后执行，策略冒烟不能替代包内容检查。

`tests/smoke/store-listing.ps1` 验证简体中文和英文 Store JSON 都满足 Partner Center 的描述、短描述、功能、关键词、系统要求、许可与 HTTPS URL 限制，并要求两份可读文档逐段、逐功能包含结构化 JSON 的权威文案。负向样本证明超长功能、过多关键词及描述中的 URL 会被拒绝。该门禁验证文字材料，不替代截图、年龄分级、正式 Identity 或认证。

同一冒烟还验证截图原子导入：动态生成八张合格 PNG，要求完整采集元数据并从独立目录导入，随后再次运行正式清单验证器；缺少元数据、低分辨率、重复内容和覆盖已有素材都会失败。临时素材最终清理，不会进入正式目录。

`tests/helpers/msix-repair` 是测试专用的 C# 控制台辅助程序，主产品仍以 Rust 实现。CI 以锁定的 Windows App SDK 1.8 依赖编译，并由不加载 Windows App SDK 的 PowerShell 监护脚本启动无副作用 `--probe`；因此即使 App SDK 在 helper 入口前阻塞，监护脚本仍会在 15 秒后直接终止该进程，工作流另有 2 分钟外层上限。Runner 不返回时明确记录查询未完成/不支持，不会一直挂起或伪报 Repair 通过；1 秒阻塞夹具持续验证这条硬超时路径。可信包生命周期支持 Repair 时，在包 LocalState 写入随机哨兵，调用 `RepairPackageAsync` 后要求内容不变，再要求 `Reset-AppxPackage` 删除哨兵；不支持时证据明确记为 `unsupported`。

`tests/performance/desktop-baseline.ps1` 对优化后的两个桌面程序各启动五次，计时到原生窗口可响应，并在 1.5 秒稳定期后汇总根进程与后代进程内存。参考机器为 Windows 11 build 26200、i9-14900HX：Iced 启动中位数 668.79 ms、工作集 225.87 MiB、私有内存 265.05 MiB；Dioxus/WebView2 候选分别为 294.25 ms、405.91 MiB、206.62 MiB。首轮冷启动包含在 p95 中（Iced 1785.51 ms、候选 644.20 ms）。工作集与私有内存口径不同，该数据是同机回归基线，不代表完整内容就绪、真实十万项归档的精确首屏/滚动延迟或跨机器门禁。
