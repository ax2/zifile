---
title: Microsoft Store 上架文案
description: 可粘贴到 Partner Center 的 ZiFile 简体中文元数据与素材状态。
---

本页是 `packaging/store/listing.zh-CN.json` 的可读副本。JSON 是字段限制自动校验的权威来源；提交前应从 Partner Center 导出最新模板并再次核对字段。

## 定位与短描述

产品名：ZiFile

类别：实用工具与工具

定价：免费；无广告、内购或订阅

短描述：面向 Windows 10 和 Windows 11 的现代开源压缩工具。浏览并安全解压 RAR、CAB、ZIP、7z 和 TAR，创建开放格式，支持加密和命令行。

## 完整描述

ZiFile 是一款面向 Windows 的现代开源压缩与归档工具，提供清晰的操作流程和谨慎的安全默认值。

你可以浏览压缩包内容，只解压选中的项目，或把文件和文件夹创建为 ZIP、7z、TAR 以及常用的压缩流。RAR 1.3 至 RAR 7 和 Windows CAB 支持浏览、校验和解压，但不能创建。ZIP 和 7z 支持 AES 加密；密码不会写入日志或设置。

ZiFile 会阻止路径穿越、链接逃逸、Windows 保留名称和不安全的覆盖，并对条目数量与解压膨胀设置上限。耗时任务在隔离的后台进程中运行，可以查看进度并随时取消。

应用提供简体中文和英文界面，支持拖放、文件关联和命令行工具。所有归档处理均在本机完成，不需要账号、云服务、广告或遥测。

ZiFile 采用 MIT 许可证开源，适用于 x64 和 ARM64 Windows 设备。

## 功能与关键词

Partner Center 会自动为功能添加项目符号，粘贴时不要自行加入符号。

1. 浏览和解压 RAR、CAB、ZIP、7z、TAR 及常用压缩流
2. ZIP 与 7z AES 加密，密码不写入日志或设置
3. 防止路径穿越、链接逃逸、危险覆盖和解压膨胀
4. 隔离后台任务，提供进度显示和随时取消
5. 拖放、文件关联和命令行工具
6. 简体中文和英文界面，支持 x64 与 ARM64

关键词：压缩、解压缩、ZIP、7z、CAB、归档、文件工具。

首次提交时“此版本新增内容”留空。适用许可条款填写 `MIT License`，Developed by 填写 `ZiCode`。

## URL 与认证备注

- 支持：`https://github.com/ax2/zifile/issues`
- 网站：`https://ax2.github.io/zifile/`
- 隐私：`https://ax2.github.io/zifile/product/privacy/`

认证备注应说明：应用只处理用户选择的本机文件；长任务由随包安装的隔离 Worker 执行；资源管理器扩展只启动可见桌面的创建或“解压到同名目录”流程，本身不解析归档、不执行压缩且不接触密码；密码通过内存 IPC 或标准输入临时传递；RAR 仅支持读取、不支持创建；MSIX 未声明 Internet 客户端能力。

## 素材状态

MSIX 包内图标已经自动生成并接受包审计。Partner Center 建议使用的 300×300 1:1 App tile listing 图标也已保存到 `packaging/store/listing-assets/`，其尺寸、格式、固定哈希和生成器一致性由 CI 检查。商店至少要求一张 PC 截图，微软建议至少四张；正式提交前仍需从签名候选包采集简体中文和英文的“主页/打开压缩包”“创建压缩包”“解压选项”“后台进度或完成状态”截图，并记录视口、缩放、主题和版本。当前文案与图标准备完成不等于商店提交完成。
