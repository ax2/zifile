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

固定负向矩阵为全部 15 类受支持格式构造带正确签名或扩展提示、但截断/损坏的最小输入，并要求 List 与完整性校验都返回普通错误而不 panic。它补充而不替代持续 libFuzzer campaign、历史 7z 崩溃夹具和第三方真实语料。

基础冒烟还会向真实 `zifile-worker.exe` 发送版本化 list 请求，要求最终完整进度快照先于 metadata、Unicode entry 和唯一结束事件；随后对 32 MiB 随机输入启动 7z 创建并发送取消控制消息，验证 Worker 在时限内退出、目标不存在且没有临时文件残留。桌面协议单测覆盖缺少终结事件和逐条条目重建；Windows 实机检查验证桌面可通过 Job Object Worker 打开并校验 7z。

列出和完整性校验分别使用兼容旧入口的 `ListOptions` 与 `TestOptions`，为 ZIP、7z、RAR、CAB、TAR 组合和六种压缩流提供统一进度与协作取消。列出阶段按扫描条目推进，单压缩流还按实际解码字节反馈；无法预知总数时两套 UI 显示“正在扫描”，完成后再发布一致的最终总量。Worker 每 100 ms 发送有界进度，并在操作返回后补发最终快照，避免小归档只出现初始 0%；预取消回归要求在解析前返回 `Cancelled`，各格式往返测试同时验证最终进度不变量。

CLI 密码单测覆盖显式 opt-in、CRLF/LF 删除、前后空格保留和缺失/空输入拒绝。基础冒烟还要求 CLI 帮助仅暴露 `--password-stdin`，并通过标准输入真实创建、校验和解压 AES 7z；固定测试密码不输出到结果。

压缩等级契约单测覆盖各编码器的有效范围、核心层边界钳制和固定等级格式。CLI 单测要求 `zifile formats` 公开每种格式的区间、`fixed` 或 `none`，并在创建前拒绝格式级越界值和固定格式的显式等级，避免命令行输入被核心层静默钳制或忽略。基础冒烟还会启动真实 `zifile.exe`，核对能力表关键行、ZIP 10 被拒绝、TAR 显式等级被拒绝且省略等级可创建。7z 集成回归分别创建等级 0 与 9 的归档，读取公开的 LZMA2 coder properties 并要求两者不同，从归档元数据证明 UI/CLI 传入的等级确实生效，而不是只验证参数经过 Worker。

任务栏状态映射使用纯单元测试覆盖隐藏、不确定、正常和取消状态；x64 Release/MSIX 构建验证 COM 绑定和 App Execution Alias manifest。任务栏视觉检查与 Narrator/UI Automation 检查仍需在适合的交互式测试环境补证。

共享操作队列单元测试覆盖首项立即启动、后续严格 FIFO、32 项容量边界、满队列不丢失返回载荷、陈旧完成 ID 不推进、清空只影响等待项、等待载荷立即释放，以及 `Debug` 输出不包含载荷。Iced 与 Dioxus 均编译接入同一调度器；在真实前台回合完成“运行中提交至少两项、取消当前项、后续继续、清空等待项”前，不把路线图条目标为完成。

`tests/performance/operation-queue-foreground.ps1` 使用 100,000 条目 ZIP 在真实前台会话验证 FIFO、取消当前项、继续下一项、清空等待项和 Worker 回收；它支持默认 Iced 与可访问 Dioxus 候选并同时识别中英文 UI。`tests/performance/extraction-cancellation-foreground.ps1` 使用确定大小的多条目 ZIP，通过 `--extract-here` 启动真实解压取消流程，检查 Worker 退出，并确认已提交的文件都是完整条目大小、没有部分输出文件。两者都必须在未被用户占用的交互桌面运行；缺少语义 Document UIA 树或未运行前台会话时，证据保持未完成。

核心测试 `active_cancellation_does_not_commit_a_partial_zip_output` 在首个解压进度数据块后取消 ZIP 解压，要求返回 `Cancelled` 且原子目标文件不存在；本地重复五次通过。该单测增强了核心层证据，但不能替代真实桌面队列验收。

性能门禁需要多轮运行和稳定基线，不能用一次共享 CI 结果直接判定回归。

`tests/reproducibility/windows-build.ps1` 在两个全新目标目录使用固定 Rust 1.93.0、锁文件、单作业构建与 MSVC `/Brepro`，比较默认桌面、可访问候选、CLI、Worker 和 Explorer DLL。云端 32813453959 在新工具链上得到双架构各 4/5；schema v2 运行 32822543635 进一步确认默认 Iced EXE 的 `.rdata` 首差异是 `glutin_wgl_sys` 生成绑定内嵌的 `build-a`/`build-b` target 路径。脚本现用 `CARGO_ENCODED_RUSTFLAGS` 与 `--remap-path-prefix` 把两个根映射到同一虚拟路径；修复后 32826187552 的 x64/ARM64 都达到 5/5 且 `reproducible=true`。

Windows CI 使用 PowerShell `Compress-Archive`/`Expand-Archive` 和系统 `tar.exe`，分别对 ZIP、tar.gz 与 7z 执行双向互操作：参考工具创建的包由 ZiFile 校验和解压，ZiFile 创建的包由参考工具解压并逐文件核对（含 Unicode 路径）。

新增的官方 7-Zip 语料门禁使用 GitHub Windows Runner 上的 `7z.exe`，覆盖 Copy、LZMA、LZMA2+BCJ、Deflate、BZip2、PPMd，以及带文件名加密的 LZMA2+AES；反向还要求官方 7-Zip 校验并解压 ZiFile 创建的普通与 AES 归档。所有场景逐文件比较 SHA-256，并上传不含密码的 JSON 证据。CI 32836336921 使用 7-Zip 26.02 完成 9/9 场景，证据 JSON SHA-256 为 `06278BB8B96AB683A3C117BA5E30F1B4AB1CF89F1BBF01E72BAC0CC26B49DB14`。

RAR 门禁从固定的 `rars` 源码提交 `7d8f9386ef777a2415da34fe1db193d8471ff7d0` 下载六个夹具，使用硬编码 SHA-256 验证来源后，逐文件比较 ZiFile 与 7-Zip 的解压树。覆盖 RAR 1.3、1.54 多文件、RAR 3 PPMd、RAR 5 压缩与 E8E9 过滤，以及 WinRAR 7.21 加密头/Quick Open；另有三个链接/重定向夹具必须在无输出的情况下拒绝。CI 32853686537 完成全部六个有效场景和三个拒绝场景，证据 JSON SHA-256 为 `4C52D0240B911609C7DDB0CACB2E484F56C8F886E216347603B228261C4EE8EF`。RAR 1.3 因现代 7-Zip 不再读取，改与同一固定上游提交中的已知正确解压树逐文件核对，其余五种有效归档继续与 7-Zip 26.02 交叉验证。

CAB 互操作门禁在 Windows Runner 使用系统 `makecab.exe` 生成 MSZIP 与 LZX Cabinet，再要求 ZiFile 完成签名识别、浏览、校验和解压，并与系统 `expand.exe` 的输出比较 SHA-256。None 压缩由 Rust 集成夹具覆盖；Quantum 与跨 Cabinet 集合明确不支持。每次 CI 上传不含用户数据的结构化 JSON 证据。

CAB 解码阶段负向回归保留合法元数据并翻转首个 CFDATA 压缩字节，先证明列表仍能读取单个条目，再要求完整性校验失败、选择性解压报错且目标目录为空。该检查证明损坏负载不会越过临时文件提交边界，不把只测损坏头误当成解码器覆盖。

修改时间测试先为源文件和目录设置确定时间，创建 ZIP、7z 与 TAR 家族归档，再要求列表元数据和解压后的文件/目录均保留预期值；独立创建的 RAR 5 与 CAB 夹具覆盖只读 Provider。协议测试会反序列化没有可选时间字段的旧版 `archive_entry` 事件，两套桌面程序还共用格式化测试，明确区分 UTC 与归档格式未保存时区的时间。

每次 CI 会编译 libFuzzer 目标；每周定时工作流对路径策略、格式识别和归档解析器各运行 180 秒有界 fuzz，失败时保留崩溃产物 14 天。归档目标用带格式签名的变异输入覆盖 ZIP、7z、RAR、CAB、四种 TAR 组合和六种单流格式，限制输入为 256 KiB、RSS 为 2 GiB、单输入为 10 秒；目标内部还使用 256 条目、4 MiB 展开量、64 倍压缩比和 32 层路径的严格限制。Windows/MSVC 本地 `cargo-fuzz` 因 `sevenz-rust2` DLL 与 libFuzzer 入口点参数冲突无法链接，Rust 编译检查可通过；动态 campaign 以 Linux GNU 定时工作流为验收环境。损坏、截断、炸弹及更多 libarchive 变体仍需继续扩展。

首轮归档解析器 campaign 32733658052 在约 569,000 次执行后发现 292 字节输入可令 `sevenz-rust2` 0.20.2 的文件数量分配触发 `capacity overflow`。第二轮 campaign 32803785688 又发现 173 字节输入可触发 ASan 超大内存分配；这类 OOM 不能由 `catch_unwind` 可靠隔离。两份输入均已转为永久集成测试和 fuzz 启动重放夹具。项目因此升级到 Rust 1.93.0 与带有元数据计数边界修复的 `sevenz-rust2` 0.22.0；Provider 的 panic 边界继续作为纵深防御。升级后的定向 campaign 32813469578 强制重放两份样本，并在 181 秒内继续执行 498,937 次、峰值 RSS 370 MiB，未产生新崩溃产物。

`libfuzzer-sys` 默认 panic hook 会在 unwind 前 abort，绕过 Provider 的错误边界。归档 fuzz 目标只在自身进程初始化时移除这个 hook；libFuzzer 外层仍会令任何逃出 ZiFile 的 panic 失败。目标启动时必定重放上述固定样本，防止新 campaign 因随机语料未再次命中而产生假通过。

当前 8 MiB 可压缩语料在本地 Windows x64 的一次基线中，ZIP 创建约为 262–275 MiB/s，完整性校验约为 3.04–3.15 GiB/s。该结果只用于建立初始量级，不作为跨机器 CI 阈值。

RAR 校验基准使用确定性的 8 MiB RAR 5 method-3 归档，并加入低频伪随机噪声，在保留压缩工作的同时不超过默认 `1000:1` 膨胀保护。首次本地 Windows x64 基线为 58.12–64.49 ms，即 124.06–137.65 MiB/s；它只用于同机回归，不是通用性能承诺。最初的高度周期性夹具被安全比率按设计拒绝，没有为了跑分绕过该保护。

桌面列表回归测试使用 100,000 个模拟条目，断言搜索结果正确且每次只构造最多 500 个可见行。默认 Iced 与 Dioxus 候选共用 `entry_view` 实现；运行 `cargo bench -p zifile-desktop --bench entry_browser --locked` 可复测。2026-08-24 的 Windows x64 基线中，选择性计数为 16.90–17.62 ms（5.68–5.92 M 条/秒），收集有界页为 15.46–15.96 ms（6.27–6.47 M 条/秒）。Windows 实机还以 100,000 个空条目的真实 ZIP 通过隔离 Worker 打开：UI Automation 报告 100,000 项和 200 页，但表格仅暴露当前 500 行；搜索 `99999` 精确定位末项，搜索 `000` 形成 3 页并可导航到第 3 页。

排序回归覆盖目录优先、升降序、缺失修改时间末尾、切换后回到第一页和 500 行上限。Criterion 另把全部 100,000 条按名称降序后收集一页；本机 Windows x64 初始结果为 13.96–15.32 ms（6.53–7.17 M 条/秒）。表头辅助测试要求可见方向箭头与 Dioxus `aria-sort` 状态一致。

目录浏览回归覆盖显式与隐式目录、根目录和嵌套层的直接子项、可导航面包屑、跨目录搜索，以及进入目录后清空搜索并回到第一页。10 万条目夹具在根层只合成一个目录，进入后无论排序方向都只收集当前 500 行。本机 Windows x64 初始基线中，扫描 10 万路径并合成根目录为 18.60–19.44 ms，进入目录后按名称降序并收集 500 行为 38.04–38.74 ms。

目录选择回归覆盖一次扫描得到每个直接子目录的全选/部分/未选计数、只增删目标目录后代、空目录禁用语义，以及文件与隐式目录同路径时目录行优先。Dioxus 源码门禁要求 mixed 状态、双语已选/总数标签和带页内序号的确定性行键。本机 Windows x64 对 10 万条目、半数已选集合的一次根目录聚合为 30.97–32.75 ms。

`tests/performance/archive-browser-baseline.ps1` 会确定性生成并在结束后删除 100,000 条目 ZIP，启动匹配的优化版候选和 Worker，使用 UI Automation 等待条目数与页码、滚动到 50%、翻到下一页，并以 25 ms 间隔采样根进程及后代在同一采样时刻的内存。五轮正式基线的窗口启动中位数/p95 为 258.07/308.54 ms，首个归档内容为 3373.80/3668.80 ms，50% 滚动为 195.16/246.34 ms，下一页为 805.87/1143.66 ms；同时刻整树工作集/私有内存最大值为 669.18/455.71 MiB，最多 9 个进程。首内容、滚动和翻页数据包含 UI Automation 观察开销，只用于同机回归，不作为用户可感知延迟的纯应用内测量。

`tests/performance/archive-load-cancellation.ps1` 使用相同的确定性 100,000 条目 ZIP，在候选界面暴露已启用的取消按钮后立即调用，要求最终 live status 为取消错误、不得出现成功打开状态、对应 Worker 必须退出，并在结束时关闭测试实例和删除临时 ZIP。最终五轮基线的取消完成中位数/p95 为 930.78/1088.73 ms；五轮确认时 Worker 数均为 0。该计时从 UI Automation 调用取消开始，到界面收到 Worker 的最终取消结果为止。

`tests/accessibility/keyboard-form.ps1` 从候选原生窗口根开始发送真实 Tab/Shift+Tab/Enter 和表单按键，并通过 UI Automation 读取 WebView2 内部焦点。它验证首页→归档→创建→主题→语言顺序、反向导航、归档/创建页激活、创建页 disabled 按钮不获得焦点、格式选择为 7z、压缩等级 `6→7→6`、密码键入后用 Ctrl+A/Backspace 清空，以及两个来源按钮可达。固定测试密码只在进程内使用，JSON 不记录其值。脚本在每次发送前要求前台原生句柄精确属于本次 ZiFile；用户切换到其他窗口时立即失败并在 `finally` 中关闭测试实例。`-ToggleLanguageBeforeTest` 可用 UIA 建立另一语言前置状态，并在成功或失败清理路径恢复原设置；英文与中文核心流程均已通过。双条目归档页流程的扩展正在实现，只有完成独立真实前台运行后才会加入通过证据。

`tests/performance/operation-queue-foreground.ps1` 生成 10 万条目归档，在真实窗口中提交三次完整性校验，并验证取消当前、下一项启动、清空等待队列与 Worker 回收。脚本用独立的 3 秒有界窗口激活重试证明 ZiFile 原生句柄正是 Windows 前台窗口；无法取得前台资格时拒绝运行，不能把后台 UIA 调用记为真实前台证据。等待文案超时时会附带最多 500 个字符的最后可见 Document 文本，且仍在 `finally` 中关闭进程并删除临时夹具。

`tests/smoke/packaging-policy.ps1` 在每次 Windows CI 中动态解析当前二十七个发行、语料与仓库政策 PowerShell 脚本，并验证缺失/部分 Partner Center Identity、非法 Name/X.500 Publisher、缺失云签名输入、非法 provider、开发 Identity、未签名 OID Publisher、无效签名产物和未完成的 1.0 就绪清单都会被拒绝，形式正确的输入及 11/11 带证据就绪夹具会被接受；它还要求签后审计、仅签后发布、最小权限、签名超时/并发控制、轮换/应急停止/吊销运维手册，以及版本、发布说明、贡献者、安全和发布就绪门禁均接入 CI。所有 CI、文档部署、SBOM 和 GitHub Release 发布 Job 都有按工作负载设置的硬超时，并由 Job 作用域检查防止误命中同文件其他上限。双架构可复现构建另受 120 分钟硬超时、同分支旧任务取消、矩阵独立结论和失败证据保留约束保护；前台队列脚本的真实窗口所有权与有界诊断也受防退化检查。真实账号、云 HSM 签名和 x64/ARM64 包内容审计不能由策略冒烟替代。

`tests/smoke/store-listing.ps1` 验证简体中文和英文 Store JSON 都满足 Partner Center 的描述、短描述、功能、关键词、系统要求、许可与 HTTPS URL 限制，并要求两份可读文档逐段、逐功能包含结构化 JSON 的权威文案。负向样本证明超长功能、过多关键词及描述中的 URL 会被拒绝。该门禁验证文字材料，不替代截图、年龄分级、正式 Identity 或认证。

同一冒烟还验证截图原子导入：动态生成八张合格 PNG，要求完整采集元数据并从独立目录导入，随后再次运行正式清单验证器；缺少元数据、低分辨率、重复内容和覆盖已有素材都会失败。临时素材最终清理，不会进入正式目录。

`tests/helpers/msix-repair` 是测试专用的 C# 控制台辅助程序，主产品仍以 Rust 实现。CI 以锁定的 Windows App SDK 1.8 依赖编译，并由不加载 Windows App SDK 的 PowerShell 监护脚本启动无副作用 `--probe`；因此即使 App SDK 在 helper 入口前阻塞，监护脚本仍会在 15 秒后直接终止该进程，工作流另有 2 分钟外层上限。Runner 不返回时明确记录查询未完成/不支持，不会一直挂起或伪报 Repair 通过；1 秒阻塞夹具持续验证这条硬超时路径。可信包生命周期支持 Repair 时，在包 LocalState 写入随机哨兵，调用 `RepairPackageAsync` 后要求内容不变，再要求 `Reset-AppxPackage` 删除哨兵；不支持时证据明确记为 `unsupported`。

`tests/smoke/wack-readiness.ps1` 用不签名开发包夹具证明 readiness 会报告 WACK 缺失、签名无效、开发 Identity、未签名 Publisher、最低系统版本错误和包/审计哈希不一致，并验证 `-RequireReady` 失败时仍保存结构化证据。该测试不运行 WACK，也不替代正式签名候选的认证报告。

`tests/performance/desktop-baseline.ps1` 对优化后的两个桌面程序各启动五次，计时到原生窗口可响应，并在 1.5 秒稳定期后汇总根进程与后代进程内存。参考机器为 Windows 11 build 26200、i9-14900HX：Iced 启动中位数 668.79 ms、工作集 225.87 MiB、私有内存 265.05 MiB；Dioxus/WebView2 候选分别为 294.25 ms、405.91 MiB、206.62 MiB。首轮冷启动包含在 p95 中（Iced 1785.51 ms、候选 644.20 ms）。工作集与私有内存口径不同，该数据是同机回归基线，不代表完整内容就绪、真实十万项归档的精确首屏/滚动延迟或跨机器门禁。
