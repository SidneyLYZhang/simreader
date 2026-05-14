# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.5] - 2026-05-14

### Added

- `rows` 命令：按行号查看文件指定行数据，支持文本文件与数据文件的精确行选取
- `--rows` / `-r <ROW>` 参数：支持指定行号列表（如 `2,9,101`）或行号范围（如 `3:7`）查看指定行
- `--txt` 参数：配合 `--rows` 使用，强制以纯文本模式输出，忽略配置的行宽限制
- 数据文件 `rows` 输出自动附加行索引列（Row），便于定位原始数据位置
- 数据文件 `rows` 输出支持列宽自动截断（超长内容以 `...` 省略），提升可读性

## [0.3.2] - 2026-05-13

### Fixed

- 修复 Linux musl 目标编译失败：`keyring` 依赖 `libdbus-sys` 在 musl 环境下不可用 #?
- `Cargo.toml` 中将 Linux `keyring` 的编译条件由 `target_os = "linux"` 改为排除 `target_env = "musl"`
- `src/config/mod.rs` 中为 musl 目标添加条件编译回退，keyring 相关方法返回明确的错误提示
- CI 工作流增加 Linux 系统依赖安装步骤（`libdbus-1-dev`、`pkg-config`），修复 `x86_64-unknown-linux-gnu` 构建
- Release 工作流增加 musl 工具链安装（`musl-tools`）

## [0.3.1] - 2026-05-13

### Added

- 多命令 CLI 架构，支持 `head`、`tail`、`schema`、`summary`、`chat`、`config` 子命令
- 多格式数据文件读取：CSV/TSV、JSON/NDJSON/JSONL、Parquet、Arrow IPC、Excel (xlsx/xls/ods)
- 文本文件读取支持，基于行索引的 O(1) 随机访问
- `head` / `tail` 命令：查看文件头部或尾部数据，支持行数、列选择、CSV 分隔符配置
- `schema` 命令：查看文件结构、列名、数据类型与统计信息（均值、中位数、最大/最小值等）
- `summary` 命令：详细的逐列统计（计数、空值、零值、均值、中位数、标准差、方差、偏度、峰度等）
- LLM 驱动的智能摘要与分析，自动生成段落级与全文总结
- `chat` 命令：与 LLM 进行关于文件内容的交互式问答，提供交互式 REPL 模式与单次问答模式
- 短格式 CLI 语法：`-s`（摘要）、`-h`（头部）、`-t`（尾部）、`-e`（结构）、`-q`（问答）
- `config` 命令：查看与修改 LLM 供应商、模型、API 地址、API 密钥、思考模式、显示行宽、输出语言等配置
- 支持 DeepSeek 和 OpenRouter 两种 LLM 供应商
- 思考/推理模式支持，可配置推理强度（low/medium/high/xhigh/max）
- API 密钥通过系统密钥环安全存储（Windows/macOS/Linux），不写入配置文件
- 配置文件支持（TOML 格式），按操作系统存放在标准配置目录
- 灵活的列选择功能，支持按列名、索引范围（如 `0:5`）或索引列表（如 `2,4,7`）筛选
- 可配置的终端行宽与 LLM 输出语言

### Dependencies

- **polars** (`0.53`)：高性能 DataFrame 处理
- **calamine** (`0.34`)：Excel / ODS 文件解析
- **clap** (`4.6`)：命令行参数解析
- **reqwest** (`0.13`) + **tokio** (`1`)：异步 HTTP 请求
- **keyring** (`3`)：跨平台密钥存储
- **serde** / **serde_json** (`1`)：序列化与反序列化
- **toml** (`0.8`)：配置文件解析
- **dirs** (`5`)：标准系统目录定位
- **regex** (`1`)：正则表达式支持
- **unicode-width** (`0.2`)：Unicode 字符宽度计算
- **aws-lc-rs**：加密支持（通过依赖间接引入）
- **brotli**、**simd-json**：数据压缩与高性能 JSON 解析（通过 polars 间接引入）
- **dbus-secret-service**：Linux 密钥环后端（通过 keyring 间接引入）

[0.3.5]: https://github.com/SidneyLYZhang/simreader/releases/tag/v0.3.5
[0.3.2]: https://github.com/SidneyLYZhang/simreader/releases/tag/v0.3.2
[0.3.1]: https://github.com/SidneyLYZhang/simreader/releases/tag/v0.3.1
