---
title: 可复现 Windows 构建
description: 固定工具链、确定性链接和双构建 SHA-256 对比方法。
---

ZiFile 将“同一源码可以再次编译”与“产物逐字节相同”分开验证。仓库通过 `rust-toolchain.toml` 固定 Rust 1.93.0；Windows Release 还固定为单作业构建并向 MSVC 链接器传递 `/Brepro`。依赖版本必须来自已提交的 `Cargo.lock`，所有命令使用 `--locked`。双构建脚本还用 `CARGO_ENCODED_RUSTFLAGS` 为每个隔离的 `CARGO_TARGET_DIR` 注入 `--remap-path-prefix=FROM=Z:\zifile-target`，避免依赖生成的 Rust 源文件把 `build-a`/`build-b` 绝对路径编入 panic 位置。

## 本机复测

在仓库根目录运行：

```powershell
./tests/reproducibility/windows-build.ps1 -Architecture x64
```

脚本在系统临时目录创建两个彼此隔离、名称不同的 Cargo 目标目录，分别执行完整 Release/all-features 构建。它比较以下五个文件的 SHA-256：

- `zifile-desktop.exe`
- `zifile-desktop-accessible.exe`
- `zifile.exe`
- `zifile-worker.exe`
- `zifile_shell.dll`

结果写入 `target/reproducibility-x64.json`，包含提交 SHA、工作区是否有未提交修改、编译器版本、目标三元组、精确命令和两组哈希。schema v2 还会为不同的 PE 及每个不同组件记录首个差异、组件内偏移和前后 64 字节有界十六进制上下文，并逐项比较 headers、各 section 与 overlay 的范围和 SHA-256；这些诊断足以区分链接布局、代码、只读数据、资源和尾部数据差异，而不必保留两份大型构建目录。任何文件不同都会令脚本失败；临时构建目录无论成功或失败都会在安全路径检查后删除。

已有 PE 可用诊断模式直接比较，不执行 Cargo 构建：

```powershell
./tests/reproducibility/windows-build.ps1 `
  -ComparePeFirstPath first\zifile-desktop.exe `
  -ComparePeSecondPath second\zifile-desktop.exe
```

ARM64 使用同一脚本和 `-Architecture arm64`。每月计划任务、手动运行以及影响工具链/构建门禁的 Pull Request 会分别复测 x64 与 ARM64，并保留结构化 JSON 30 天；无关 PR 不承担这项长时构建成本。

## 当前证据

2026-08-24 基于 Rust 1.88.0 的本地 Windows x64 完整双构建中，可访问候选、CLI、Worker 与 Explorer DLL 的两组 SHA-256 完全相同；默认 `zifile-desktop.exe` 不同。云端运行 `32707399686` 随后在干净 PR 合并提交上复现了相同结论：x64 与 ARM64 都是 4/5 匹配，只有默认 Iced EXE 不同，因此两项总体结果均为 `reproducible=false`，路线图保持未完成。此前的小范围调查确认 `/Brepro` 能消除 PE 时间戳/调试标识差异，单作业能让 CLI 稳定复现；Iced/WGPU 默认桌面路径仍有额外非确定性，需要继续定位。

2026-08-25 为采用已修复解析器后端把固定工具链升级到 Rust 1.93.0。云端运行 `32813453959` 在干净 PR 合并提交上重新建立证据：x64 与 ARM64 仍各有 4/5 PE 匹配，只有默认 Iced EXE 不同。schema v2 运行 `32822543635` 随后定位了两个架构的同一根因：`.rdata` 首差异均是 `glutin_wgl_sys` 生成绑定中的 `build-a` 对 `build-b` 隔离 target 路径；`.text` 和其他已检查 section 相同，header 差异是 `/Brepro` 内容哈希随 `.rdata` 变化的后果。加入路径重映射后，运行 [`32826187552`](https://github.com/ax2/zifile/actions/runs/32826187552) 首次得到 x64 与 ARM64 均 5/5；两份 JSON 都为 `reproducible=true`，因此同 Runner/提交/工具链的裸 PE 门禁已完成。

## 已知边界

该门禁证明同一 Windows Runner、同一提交和同一锁定工具链下的裸 PE 文件逐字节一致。代码签名会加入签名数据，MSIX 打包还包含容器元数据，因此签名包不能直接用裸 PE 哈希结论替代双包比较。跨 Visual Studio/MSVC 版本、跨机器和签名后 MSIX 的可复现性仍需单独验证。

单作业不是性能优化，而是确定性约束。本机调查发现并行原生依赖构建会造成少量代码布局差异；仅加入 `/Brepro` 不足以消除这项差异。正式 Release 使用相同约束，避免验证脚本与发布流程采用不同构建模型。当前这组约束仍未使 Iced EXE 通过，不得将 4/5 的结果描述为“ZiFile Windows 构建已完全可复现”。
