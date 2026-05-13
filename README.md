# SimReader

![Crates.io License](https://img.shields.io/crates/l/simreader)
![Crates.io Version](https://img.shields.io/crates/v/simreader)
![GitHub Release Date](https://img.shields.io/github/release-date/SidneyLYZhang/simreader)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/SidneyLYZhang/simreader/rust.yaml)


A Simple Reader for data files and text files in Rust.

## Features

- **Multi-format data reading**: CSV/TSV, JSON/NDJSON/JSONL, Parquet, Arrow IPC, Excel (xlsx/xls/ods)
- **Text file reading**: Line-indexed high-performance reader with O(1) random access
- **Data exploration**: Head, tail, schema statistics, and detailed column summaries
- **LLM-powered analysis**: Interactive Q&A and AI-generated summaries via DeepSeek or OpenRouter
- **Flexible output**: Configurable line wrapping, column selection, and multi-language LLM output
- **Secure credential storage**: API keys stored in system keyring

## Installation

```bash
cargo install simreader
```

Pre-built binaries may also be available from the [releases page](https://github.com/SidneyLYZhang/simreader/releases).

## Quick Start

```bash
# View the first 10 lines of a CSV
simreader head data.csv -n 10

# View data schema and column statistics
simreader schema data.csv

# Get a detailed summary
simreader summary data.csv
```

## Supported File Formats

| Format   | Extensions                 | Engine       |
|----------|---------------------------|--------------|
| CSV/TSV  | `.csv`, `.tsv`            | Polars       |
| JSON     | `.json`, `.ndjson`, `.jsonl` | Polars    |
| Parquet  | `.parquet`                | Polars       |
| Arrow IPC| `.ipc`, `.arrow`          | Polars       |
| Excel    | `.xls`, `.xlsx`, `.ods`   | Calamine     |
| Text     | All other extensions       | Built-in     |

The file format is auto-detected from the extension. Use `--csv` to force CSV mode for files without a recognized extension.

## Commands

### `head` — View the beginning of a file

```bash
simreader head <FILE> [OPTIONS]
```

| Option        | Description                           |
|---------------|---------------------------------------|
| `-n, --num N`  | Number of rows to display (default: 5) |
| `--no-name`    | Use column index instead of names     |
| `--csv`        | Force CSV reading mode                |
| `-s, --separator SEP` | CSV delimiter (with `--csv`, default: `,`) |
| `--col COLS`   | Column selection (name, range `0:5`, or list `2,4,7`) |

### `tail` — View the end of a file

```bash
simreader tail <FILE> [OPTIONS]
```

Same options as `head`.

### `schema` — View file schema and statistics

```bash
simreader schema <FILE> [OPTIONS]
```

| Option             | Description                                  |
|--------------------|----------------------------------------------|
| `-d, --direction DIR` | Statistics direction: `col` or `row` (default: `col`) |
| `--no-name`        | Use column index instead of names            |
| `--csv`            | Force CSV reading mode                       |
| `-s, --separator SEP` | CSV delimiter (with `--csv`)              |
| `--col COLS`       | Column selection                             |

For data files, displays column names, types, counts, and basic statistics (mean, median, min, max) for numeric columns, plus type info and top-frequency content for string columns. For text files, displays word counts (English/Chinese), line count, and paragraph count.

### `summary` — Detailed file summary

```bash
simreader summary <FILE> [OPTIONS]
```

| Option        | Description                       |
|---------------|-----------------------------------|
| `--no-name`    | Use column index instead of names |
| `--csv`        | Force CSV reading mode            |
| `-s, --separator SEP` | CSV delimiter (with `--csv`) |
| `--col COLS`   | Column selection                  |

For data files, provides in-depth per-column statistics (count, nulls, zeros, mean, median, mode, std dev, variance, skewness, kurtosis, min, max). For text files, displays word/line/paragraph statistics and, when an LLM is configured, AI-generated paragraph summaries and overall text overview.

### `chat` — Interactive LLM Q&A about a file

```bash
simreader chat <FILE> [QUESTION]
```

Engage in an interactive conversation with an LLM about file contents. If a question is provided, it runs in single-shot mode; otherwise, an interactive REPL starts (`/exit`, `/quit`, or `/q` to exit). Data files are previewed (first 100 rows, up to 50,000 chars) before being sent to the LLM.

### `config` — View or modify configuration

```bash
# View current configuration
simreader config

# Set LLM provider (deepseek or openrouter)
simreader config --provider deepseek

# Set model
simreader config --model deepseek-v4-flash

# Set API base URL
simreader config --base-url https://api.deepseek.com/

# Set API key (stored in system keyring, not in config file)
simreader config --api-key YOUR_KEY

# Enable/disable reasoning/thinking mode
simreader config --think
simreader config --no-think

# Set thinking intensity (low/medium/high/xhigh/max)
simreader config --think-intensity high

# Set display line width for text output
simreader config --line-width 100

# Set LLM output language
simreader config --language 中文
```

## Configuration File

Configuration is stored in `config.toml` at the system config directory (e.g., `~/.config/simreader/` on Linux, `~/Library/Application Support/simreader/` on macOS, `%APPDATA%/simreader/` on Windows). API keys are stored securely in the system keyring, not in the config file.

```toml
[llm]
provider = "deepseek"
model = "deepseek-v4-flash"
base_url = "https://api.deepseek.com/"

[llm.thinking]
enabled = true
effort = "max"
# max_tokens = 4096  # optional
# exclude = false

[display]
line_width = 80
output_language = "中文"
```

## LLM Providers

| Provider    | Default Model           | Default API URL                     |
|-------------|------------------------|-------------------------------------|
| DeepSeek    | `deepseek-v4-flash`    | `https://api.deepseek.com/`         |
| OpenRouter  | `moonshotai/kimi-k2.6` | `https://openrouter.ai/api/v1`      |

Both providers support reasoning/thinking mode with configurable effort levels (`low`, `medium`, `high`, `xhigh`, `max`). API keys must be obtained from the respective provider.

## Short-Form Usage

SimReader also supports a shorter syntax without explicit subcommands:

```bash
# Summary
simreader data.csv -s

# Head (first 5 rows)
simreader data.csv -h -n 5

# Tail (last 5 rows)
simreader data.csv -t -n 5

# Schema
simreader data.csv -e

# LLM Q&A
simreader data.csv -q "How many rows are there?"
```

## Thanks

- [Polars](https://pola.rs/) — High-performance DataFrame library
- [calamine](https://github.com/tafia/calamine) — Excel file reader
- [clap](https://github.com/clap-rs/clap) — Command-line argument parser

## License

MIT License

Copyright (c) Sidney Zhang <zly@lyzhang.me>
