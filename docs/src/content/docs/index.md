---
title: ZiFile
description: 面向 Windows 的现代、安全、开源文件与压缩工具。
template: splash
hero:
  tagline: 使用 Rust 从零构建的现代 Windows 压缩与文件工具
  actions:
    - text: 查看路线图
      link: /product/roadmap/
      icon: right-arrow
    - text: GitHub
      link: https://github.com/ax2/zifile
      icon: external
      variant: minimal
---

ZiFile 由 ZiCode 发起，采用 MIT 协议。首个版本聚焦安全地浏览、创建和解压主流压缩格式，长期扩展为一组可信赖的文件操作能力。

当前处于 **Stage 0 — Foundation**：仓库、Rust workspace、Iced UI 技术壳、测试体系和文档系统已经建立；实际压缩与解压能力从 Stage 1 开始交付。

## 设计原则

- **安全默认值**：压缩包始终作为不可信输入处理。
- **诚实的能力声明**：界面只展示后端明确声明的能力。
- **后台执行**：解析、压缩和解压不能阻塞 UI。
- **可追踪发布**：版本、构建、文档、SBOM 和发布记录保持一致。
- **Windows 优先**：先把 Windows 10/11 的安装、Shell 和辅助功能体验做好。
