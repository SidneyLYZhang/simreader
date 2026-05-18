//! 统一输入源抽象模块。
//!
//! 提供三种输入来源（标准输入、文件、内存字节）的统一封装，
//! 以及根据文件扩展名或 CLI 参数自动判定数据格式的能力。
//!
//! # 主要类型
//!
//! - [`InputSource`] — 输入来源枚举（stdin / file / bytes）
//! - [`DataFormat`] — 数据格式枚举（Text / Csv / Json / Ipc / Parquet / Excel）
//! - [`InputConfig`] — 组合来源与格式的配置，提供读取器工厂方法
//! - [`TextReader`] — 惰性行迭代器（行号流式）
//! - [`CsvReader`] — 惰性 CSV 记录迭代器
//!
//! # 使用示例
//!
//! ```ignore
//! let input = InputConfig::from_cli(&matches, Some("data.csv"))?;
//! match input.format() {
//!     DataFormat::Text => {
//!         let reader = input.text_reader()?;
//!         for line in reader {
//!             println!("{}", line?);
//!         }
//!     }
//!     DataFormat::Csv { .. } => {
//!         let reader = input.csv_reader()?;
//!         for record in reader {
//!             let fields: Vec<String> = record?;
//!             println!("{:?}", fields);
//!         }
//!     }
//!     _ => { /* 使用 Polars 读取 */ }
//! }
//! ```

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Read};
use std::path::{Path, PathBuf};

// ============================================================================
// 输入来源
// ============================================================================

/// 输入数据的来源
#[derive(Debug, Clone)]
pub enum InputSource {
    /// 标准输入（管道或重定向）
    Stdin,
    /// 文件路径
    File(PathBuf),
    /// 内存字节数据
    Bytes(Vec<u8>),
}

// ============================================================================
// 数据格式
// ============================================================================

/// 输入内容的解释格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    /// 纯文本，按行读取
    Text,
    /// CSV 格式（含分隔符和是否将首行作表头）
    Csv {
        /// 字段分隔符字节
        delimiter: u8,
        /// 第一行是否为表头
        has_header: bool,
    },
    /// JSON / NDJSON / JSONL
    Json,
    /// Arrow IPC
    Ipc,
    /// Apache Parquet
    Parquet,
    /// Excel（xls/xlsx/ods）
    Excel,
}

impl DataFormat {
    /// 返回 CSV 分隔符，非 CSV 格式返回 `b','`
    pub fn delimiter(&self) -> u8 {
        match self {
            DataFormat::Csv { delimiter, .. } => *delimiter,
            _ => b',',
        }
    }

    /// 是否将第一行视为表头
    pub fn has_header(&self) -> bool {
        match self {
            DataFormat::Csv { has_header, .. } => *has_header,
            _ => true,
        }
    }
}

// ============================================================================
// 统一输入配置
// ============================================================================

/// 组合输入来源和数据格式的配置对象
///
/// 从 CLI 参数中构建，并提供创建对应读取器的工厂方法。
#[derive(Debug, Clone)]
pub struct InputConfig {
    /// 输入来源
    pub source: InputSource,
    /// 数据格式
    pub format: DataFormat,
}

impl InputConfig {
    /// 从 CLI 参数创建 `InputConfig`
    ///
    /// * `matches` — clap 解析后的参数匹配
    /// * `file_arg` — 可选的文件路径（从 `FILE` 参数或位置参数获取）
    ///
    /// # 来源判定
    ///
    /// 1. 若提供了文件路径 → `File(path)`
    /// 2. 若 stdin 非终端（有管道数据）→ `Stdin`
    /// 3. 否则报错
    ///
    /// # 格式判定
    ///
    /// - 文件来源：若 `--csv` 则强制 CSV；否则按扩展名检测
    /// - Stdin/Bytes：若 `--csv` 则 CSV；否则 Text
    pub fn from_cli(matches: &clap::ArgMatches, file_arg: Option<&str>) -> anyhow::Result<Self> {
        let force_csv = matches.get_flag("csv");
        let separator = extract_separator(matches);
        let no_name = matches.get_flag("no_name");

        let source = if let Some(file) = file_arg {
            InputSource::File(PathBuf::from(file))
        } else if !std::io::stdin().is_terminal() {
            InputSource::Stdin
        } else {
            anyhow::bail!("请指定文件路径或通过管道提供输入。使用 --help 查看帮助。");
        };

        let format = match &source {
            InputSource::File(path) => {
                if force_csv {
                    DataFormat::Csv {
                        delimiter: separator.unwrap_or(b','),
                        has_header: !no_name,
                    }
                } else {
                    detect_format_from_path(path.as_path(), separator, !no_name)
                }
            }
            InputSource::Stdin | InputSource::Bytes(_) => {
                if force_csv {
                    DataFormat::Csv {
                        delimiter: separator.unwrap_or(b','),
                        has_header: !no_name,
                    }
                } else {
                    DataFormat::Text
                }
            }
        };

        Ok(InputConfig { source, format })
    }

    /// 获取文件路径（仅当来源为 `File` 时有效）
    pub fn file_path(&self) -> Option<&Path> {
        match &self.source {
            InputSource::File(path) => Some(path.as_path()),
            _ => None,
        }
    }

    /// 返回数据格式的引用
    pub fn format(&self) -> &DataFormat {
        &self.format
    }

    /// 创建文本行读取器，用于逐行迭代
    ///
    /// 适用于 `DataFormat::Text` 模式。
    pub fn text_reader(&self) -> io::Result<TextReader> {
        let inner: Box<dyn Read> = match &self.source {
            InputSource::Stdin => Box::new(io::stdin()),
            InputSource::File(path) => Box::new(File::open(path)?),
            InputSource::Bytes(data) => Box::new(io::Cursor::new(data.clone())),
        };
        Ok(TextReader {
            reader: BufReader::new(inner),
            line_buf: String::new(),
        })
    }

    /// 创建 CSV 记录读取器，用于逐条迭代
    ///
    /// 适用于 `DataFormat::Csv` 模式。
    /// 使用 [`csv`] crate 进行流式解析，支持自定义分隔符和表头设置。
    pub fn csv_reader(&self) -> anyhow::Result<CsvReader> {
        let inner: Box<dyn Read> = match &self.source {
            InputSource::Stdin => Box::new(io::stdin()),
            InputSource::File(path) => Box::new(File::open(path)?),
            InputSource::Bytes(data) => Box::new(io::Cursor::new(data.clone())),
        };
        let delimiter = self.format.delimiter();
        let has_header = self.format.has_header();

        let reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(has_header)
            .flexible(true)
            .from_reader(inner);

        Ok(CsvReader { reader })
    }

    /// 将全部内容读取为字符串
    ///
    /// 适用于需要将输入完整加载到内存的场景（如 LLM 问答）。
    pub fn read_to_string(&self) -> io::Result<String> {
        match &self.source {
            InputSource::Stdin => {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                Ok(buf)
            }
            InputSource::File(path) => std::fs::read_to_string(path),
            InputSource::Bytes(data) => Ok(String::from_utf8_lossy(data).to_string()),
        }
    }
}

// ============================================================================
// 文件扩展名 → 格式检测
// ============================================================================

/// 根据文件路径扩展名检测数据格式
fn detect_format_from_path(path: &Path, csv_separator: Option<u8>, has_header: bool) -> DataFormat {
    let lower = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let full_lower = path.to_string_lossy().to_lowercase();

    if full_lower.ends_with(".tsv") {
        return DataFormat::Csv {
            delimiter: csv_separator.unwrap_or(b'\t'),
            has_header,
        };
    }

    match lower.as_str() {
        "csv" => DataFormat::Csv {
            delimiter: csv_separator.unwrap_or(b','),
            has_header,
        },
        "json" | "ndjson" | "jsonl" => DataFormat::Json,
        "ipc" | "arrow" => DataFormat::Ipc,
        "parquet" => DataFormat::Parquet,
        "xls" | "xlsx" | "ods" => DataFormat::Excel,
        _ => DataFormat::Text,
    }
}

/// 从 clap `ArgMatches` 中提取 CSV 分隔符
fn extract_separator(matches: &clap::ArgMatches) -> Option<u8> {
    let sep_str = matches.get_one::<String>("separator")?;
    if sep_str.is_empty() {
        None
    } else {
        Some(sep_str.as_bytes()[0])
    }
}

// ============================================================================
// 文本行读取器
// ============================================================================

/// 惰性文本行读取器
///
/// 封装带缓冲的输入源，实现逐行迭代。
/// 返回的每一行已去除行尾换行符（`\n` / `\r\n`）。
pub struct TextReader {
    reader: BufReader<Box<dyn Read>>,
    line_buf: String,
}

impl Iterator for TextReader {
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.line_buf.clear();
        match self.reader.read_line(&mut self.line_buf) {
            Ok(0) => None,
            Ok(_) => {
                if self.line_buf.ends_with('\n') {
                    self.line_buf.pop();
                    if self.line_buf.ends_with('\r') {
                        self.line_buf.pop();
                    }
                }
                Some(Ok(self.line_buf.clone()))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

// ============================================================================
// CSV 记录读取器
// ============================================================================

/// 惰性 CSV 记录读取器
///
/// 封装 [`csv::Reader`]，实现逐条记录迭代。
/// 每条记录以 `Vec<String>` 返回。
pub struct CsvReader {
    reader: csv::Reader<Box<dyn Read>>,
}

impl Iterator for CsvReader {
    type Item = csv::Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut record = csv::StringRecord::new();
        match self.reader.read_record(&mut record) {
            Ok(true) => Some(Ok(record.iter().map(|s| s.to_string()).collect())),
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        }
    }
}
