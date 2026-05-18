//! 冒烟测试：覆盖输入模块、命令入口、工具函数、读取器、配置与 LLM 类型。
//!
//! 所有基于文件的测试通过 `create_temp_file` 在系统临时目录动态创建测试数据。

use simreader::commands::util;
use simreader::commands::rows::{parse_rows, RowSelection};
use simreader::config::{AppConfig, ConfigManager, ReasoningEffort};
use simreader::input::{DataFormat, InputConfig, InputSource};
use simreader::llm;
use simreader::reader;
use simreader::reader::readdata::FileFormat;

use std::io::Write;

// ============================================================================
// 测试辅助函数
// ============================================================================

fn create_temp_file(name: &str, content: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("simreader_smoke_test_files");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

/// 创建一个基于文件的 InputConfig
fn file_input(path: &std::path::Path, format: DataFormat) -> InputConfig {
    InputConfig {
        source: InputSource::File(path.to_path_buf()),
        format,
    }
}

/// 创建一个基于内存字节的 InputConfig（模拟 stdin/管道输入）
fn bytes_input(data: Vec<u8>, format: DataFormat) -> InputConfig {
    InputConfig {
        source: InputSource::Bytes(data),
        format,
    }
}

// ============================================================================
// 1. Config 模块测试
// ============================================================================

#[test]
fn smoke_config_default() {
    let config = AppConfig::default();
    assert_eq!(config.llm.provider, "deepseek");
    assert_eq!(config.llm.model, "deepseek-v4-flash");
    assert_eq!(config.llm.base_url, "https://api.deepseek.com/");
    assert!(config.llm.thinking.enabled);
    assert_eq!(config.llm.thinking.effort, Some(ReasoningEffort::Max));
    assert_eq!(config.display.line_width, 80);
    assert_eq!(config.display.output_language, "中文");
}

#[test]
fn smoke_config_toml_roundtrip() {
    let mut config = AppConfig::default();
    config.llm.provider = "openrouter".into();
    config.llm.model = "moonshotai/kimi-k2.6".into();
    config.llm.base_url = "https://openrouter.ai/api/v1".into();
    config.llm.thinking.effort = Some(ReasoningEffort::XHigh);

    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.llm.provider, "openrouter");
    assert_eq!(parsed.llm.model, "moonshotai/kimi-k2.6");
    assert_eq!(parsed.llm.base_url, "https://openrouter.ai/api/v1");
    assert_eq!(parsed.llm.thinking.effort, Some(ReasoningEffort::XHigh));
}

#[test]
fn smoke_config_manager_with_custom_path() {
    let dir = std::env::temp_dir().join("simreader_smoke_test");
    let _ = std::fs::remove_dir_all(&dir);
    let config_path = dir.join("config.toml");

    let mgr = ConfigManager::with_config_path(&config_path).unwrap();
    assert_eq!(mgr.config().llm.provider, "deepseek");
    assert_eq!(mgr.config().display.line_width, 80);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn smoke_reasoning_effort_serialization() {
    assert_eq!(
        serde_json::to_string(&ReasoningEffort::Low).unwrap(),
        r#""low""#
    );
    assert_eq!(
        serde_json::to_string(&ReasoningEffort::Max).unwrap(),
        r#""max""#
    );
}

#[test]
fn smoke_reasoning_effort_as_high_or_max() {
    assert_eq!(ReasoningEffort::Low.as_high_or_max(), "high");
    assert_eq!(ReasoningEffort::Medium.as_high_or_max(), "high");
    assert_eq!(ReasoningEffort::High.as_high_or_max(), "high");
    assert_eq!(ReasoningEffort::XHigh.as_high_or_max(), "max");
    assert_eq!(ReasoningEffort::Max.as_high_or_max(), "max");
}

// ============================================================================
// 2. LLM 类型测试
// ============================================================================

#[test]
fn smoke_chat_message_creation() {
    let sys = llm::ChatMessage::system("你是助手");
    assert_eq!(sys.role, "system");
    assert_eq!(sys.content, "你是助手");

    let user = llm::ChatMessage::user("你好");
    assert_eq!(user.role, "user");
    assert_eq!(user.content, "你好");

    let assistant = llm::ChatMessage::assistant("你好！");
    assert_eq!(assistant.role, "assistant");
    assert_eq!(assistant.content, "你好！");
}

#[test]
fn smoke_chat_request_builder() {
    let request = llm::ChatRequest {
        model: "deepseek-v4-flash".into(),
        messages: vec![
            llm::ChatMessage::system("系统提示"),
            llm::ChatMessage::user("用户问题"),
        ],
        temperature: Some(0.3),
        max_tokens: Some(2000),
        top_p: None,
        thinking: Some(llm::ThinkingConfig {
            enabled: true,
            effort: Some(ReasoningEffort::High),
            max_tokens: None,
            exclude: false,
        }),
        files: vec![],
    };

    assert_eq!(request.model, "deepseek-v4-flash");
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.messages[0].role, "system");
    assert_eq!(request.messages[1].role, "user");
    assert!(request.thinking.as_ref().unwrap().enabled);
}

#[test]
fn smoke_thinking_config_default() {
    let config = llm::ThinkingConfig::default();
    assert!(config.enabled);
    assert_eq!(config.effort, Some(ReasoningEffort::High));
    assert_eq!(config.max_tokens, None);
    assert!(!config.exclude);
}

#[test]
fn smoke_chat_request_serialization() {
    let request = llm::ChatRequest {
        model: "test-model".into(),
        messages: vec![
            llm::ChatMessage::system("sys"),
            llm::ChatMessage::user("query"),
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        top_p: Some(0.9),
        thinking: None,
        files: vec![],
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["model"], "test-model");
    assert_eq!(parsed["messages"][0]["role"], "system");
    assert_eq!(parsed["messages"][1]["role"], "user");
    assert_eq!(parsed["temperature"], 0.7);
    assert_eq!(parsed["max_tokens"], 100);
    assert_eq!(parsed["top_p"], 0.9);
}

#[test]
fn smoke_thinking_config_serialization() {
    let config = llm::ThinkingConfig {
        enabled: true,
        effort: Some(ReasoningEffort::XHigh),
        max_tokens: Some(4096),
        exclude: false,
    };

    let json = serde_json::to_string(&config).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["enabled"], true);
    assert_eq!(parsed["effort"], "xhigh");
    assert_eq!(parsed["max_tokens"], 4096);
    assert_eq!(parsed["exclude"], false);
}

// ============================================================================
// 3. Input 模块 — 格式检测
// ============================================================================

#[test]
fn smoke_detect_file_format() {
    assert!(matches!(
        util::detect_file_format("data.csv"),
        Some(FileFormat::Csv)
    ));
    assert!(matches!(
        util::detect_file_format("data.tsv"),
        Some(FileFormat::Csv)
    ));
    assert!(matches!(
        util::detect_file_format("data.json"),
        Some(FileFormat::Json)
    ));
    assert!(matches!(
        util::detect_file_format("data.ndjson"),
        Some(FileFormat::Json)
    ));
    assert!(matches!(
        util::detect_file_format("data.parquet"),
        Some(FileFormat::Parquet)
    ));
    assert!(matches!(
        util::detect_file_format("data.ipc"),
        Some(FileFormat::Ipc)
    ));
    assert!(matches!(
        util::detect_file_format("data.arrow"),
        Some(FileFormat::Ipc)
    ));
    assert!(matches!(
        util::detect_file_format("data.xlsx"),
        Some(FileFormat::Excel)
    ));
    assert!(matches!(
        util::detect_file_format("data.xls"),
        Some(FileFormat::Excel)
    ));
    assert!(matches!(
        util::detect_file_format("data.ods"),
        Some(FileFormat::Excel)
    ));
    assert!(util::detect_file_format("data.txt").is_none());
    assert!(util::detect_file_format("data.md").is_none());
}

#[test]
fn smoke_is_data_file_and_text_file() {
    assert!(util::is_data_file("data.csv"));
    assert!(util::is_data_file("data.json"));
    assert!(util::is_data_file("data.xlsx"));
    assert!(!util::is_data_file("data.txt"));
    assert!(!util::is_data_file("README.md"));

    assert!(util::is_text_file("data.txt"));
    assert!(!util::is_text_file("data.csv"));
}

#[test]
fn smoke_csv_separator_for_file() {
    assert_eq!(util::csv_separator_for_file("data.csv"), None);
    assert_eq!(util::csv_separator_for_file("data.tsv"), Some(b'\t'));
    assert_eq!(util::csv_separator_for_file("data.json"), None);
}

#[test]
fn smoke_file_format_debug() {
    assert_eq!(format!("{:?}", FileFormat::Csv), "Csv");
    assert_eq!(format!("{:?}", FileFormat::Json), "Json");
    assert_eq!(format!("{:?}", FileFormat::Ipc), "Ipc");
    assert_eq!(format!("{:?}", FileFormat::Parquet), "Parquet");
    assert_eq!(format!("{:?}", FileFormat::Excel), "Excel");
}

// ============================================================================
// 4. Input 模块 — TextReader（文本行流式读取）
// ============================================================================

#[test]
fn smoke_text_reader_from_file() {
    let path = create_temp_file("tr_file.txt", "line1\nline2\nline3\nline4\nline5\n");
    let input = file_input(&path, DataFormat::Text);
    let reader = input.text_reader().unwrap();
    let lines: Vec<String> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(lines, vec!["line1", "line2", "line3", "line4", "line5"]);
}

#[test]
fn smoke_text_reader_from_bytes() {
    let data = b"hello\nworld\n".to_vec();
    let input = bytes_input(data, DataFormat::Text);
    let reader = input.text_reader().unwrap();
    let lines: Vec<String> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(lines, vec!["hello", "world"]);
}

#[test]
fn smoke_text_reader_empty() {
    let input = bytes_input(vec![], DataFormat::Text);
    let reader = input.text_reader().unwrap();
    let lines: Vec<String> = reader.map(|r| r.unwrap()).collect();
    assert!(lines.is_empty());
}

#[test]
fn smoke_text_reader_crlf() {
    let data = b"line1\r\nline2\r\n".to_vec();
    let input = bytes_input(data, DataFormat::Text);
    let reader = input.text_reader().unwrap();
    let lines: Vec<String> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(lines, vec!["line1", "line2"]);
}

// ============================================================================
// 5. Input 模块 — CsvReader（CSV 记录流式读取）
// ============================================================================

#[test]
fn smoke_csv_reader_from_file() {
    let path = create_temp_file("cr_file.csv", "name,age\nAlice,30\nBob,25\n");
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    let reader = input.csv_reader().unwrap();
    let records: Vec<Vec<String>> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0], vec!["Alice", "30"]);
    assert_eq!(records[1], vec!["Bob", "25"]);
}

#[test]
fn smoke_csv_reader_from_bytes() {
    let data = b"col1,col2\nval1,val2\nval3,val4\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    let reader = input.csv_reader().unwrap();
    let records: Vec<Vec<String>> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0], vec!["val1", "val2"]);
    assert_eq!(records[1], vec!["val3", "val4"]);
}

#[test]
fn smoke_csv_reader_no_header() {
    let data = b"Alice,30\nBob,25\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: false,
        },
    );
    let reader = input.csv_reader().unwrap();
    let records: Vec<Vec<String>> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 2);
}

#[test]
fn smoke_csv_reader_tab_separator() {
    let data = b"name\tage\nAlice\t30\nBob\t25\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b'\t',
            has_header: true,
        },
    );
    let reader = input.csv_reader().unwrap();
    let records: Vec<Vec<String>> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0], vec!["Alice", "30"]);
}

#[test]
fn smoke_csv_reader_pipe_separator() {
    let data = b"A|B|C\n1|2|3\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b'|',
            has_header: true,
        },
    );
    let reader = input.csv_reader().unwrap();
    let records: Vec<Vec<String>> = reader.map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], vec!["1", "2", "3"]);
}

#[test]
fn smoke_csv_reader_empty() {
    let input = bytes_input(
        vec![],
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    let reader = input.csv_reader().unwrap();
    let records: Vec<Vec<String>> = reader.map(|r| r.unwrap()).collect();
    assert!(records.is_empty());
}

// ============================================================================
// 6. Input 模块 — read_to_string
// ============================================================================

#[test]
fn smoke_read_to_string_file() {
    let path = create_temp_file("rts.txt", "Hello\nWorld\n");
    let input = file_input(&path, DataFormat::Text);
    let s = input.read_to_string().unwrap();
    assert_eq!(s, "Hello\nWorld\n");
}

#[test]
fn smoke_read_to_string_bytes() {
    let input = bytes_input(b"hello world".to_vec(), DataFormat::Text);
    let s = input.read_to_string().unwrap();
    assert_eq!(s, "hello world");
}

// ============================================================================
// 7. Input 模块 — file_path / format 访问器
// ============================================================================

#[test]
fn smoke_input_config_file_path() {
    let path = create_temp_file("fp.txt", "test");
    let input = file_input(&path, DataFormat::Text);
    assert!(input.file_path().is_some());
    assert_eq!(input.file_path().unwrap().file_name().unwrap(), "fp.txt");

    let input_bytes = bytes_input(b"test".to_vec(), DataFormat::Text);
    assert!(input_bytes.file_path().is_none());
}

#[test]
fn smoke_input_config_format_accessor() {
    let input = bytes_input(
        b"".to_vec(),
        DataFormat::Csv {
            delimiter: b'|',
            has_header: false,
        },
    );
    assert_eq!(input.format().delimiter(), b'|');
    assert!(!input.format().has_header());
}

// ============================================================================
// 8. util — 词数统计与文本处理
// ============================================================================

#[test]
fn smoke_word_count_english() {
    let text = "Hello world! This is a test. How are you?";
    let wc = util::WordCount::from_text(text);
    assert_eq!(wc.en_words, 9);
    assert_eq!(wc.cn_chars, 0);
    assert_eq!(wc.total, 9);
}

#[test]
fn smoke_word_count_chinese() {
    let text = "你好世界！这是一个测试。";
    let wc = util::WordCount::from_text(text);
    assert_eq!(wc.cn_chars, 10);
    assert_eq!(wc.en_words, 0);
    assert_eq!(wc.total, 10);
}

#[test]
fn smoke_word_count_mixed() {
    let text = "Hello 世界！This is 测试。";
    let wc = util::WordCount::from_text(text);
    assert_eq!(wc.en_words, 3);
    assert_eq!(wc.cn_chars, 4);
    assert_eq!(wc.total, 7);
}

#[test]
fn smoke_word_count_empty() {
    let wc = util::WordCount::from_text("");
    assert_eq!(wc.en_words, 0);
    assert_eq!(wc.cn_chars, 0);
    assert_eq!(wc.total, 0);
}

#[test]
fn smoke_en_words_only() {
    assert_eq!(util::en_words_only("Hello, world! How are you?"), 5);
    assert_eq!(util::en_words_only("你好世界"), 0);
}

#[test]
fn smoke_cn_chars_only() {
    assert_eq!(util::cn_chars_only("Hello 你好 World 世界"), 4);
    assert_eq!(util::cn_chars_only("Hello World"), 0);
}

#[test]
fn smoke_clean_punct() {
    let cleaned = util::clean_punct("Hello, world! How are you?");
    assert_eq!(cleaned, "Hello world How are you");
}

#[test]
fn smoke_total_words() {
    assert_eq!(util::total_words("Hello 世界！"), 3);
    assert_eq!(util::total_words(""), 0);
}

// ============================================================================
// 9. util — 文本换行
// ============================================================================

#[test]
fn smoke_wrap_text_en_basic() {
    let text = "Hello world this is a test";
    let wrapped = util::wrap_text_en(text, 20);
    assert!(wrapped.contains('\n'));
}

#[test]
fn smoke_wrap_text_en_no_wrap() {
    let text = "short line";
    let wrapped = util::wrap_text_en(text, 80);
    assert_eq!(wrapped, "short line");
}

#[test]
fn smoke_wrap_text_en_empty() {
    let wrapped = util::wrap_text_en("", 80);
    assert_eq!(wrapped, "");
}

#[test]
fn smoke_wrap_text_en_leading_spaces() {
    let text = "    Hello world this is a longer test message";
    let wrapped = util::wrap_text_en(text, 20);
    assert!(wrapped.starts_with("    "));
    for line in wrapped.lines() {
        assert!(line.starts_with("    "));
    }
}

#[test]
fn smoke_wrap_text_en_zero_width() {
    let text = "Hello world";
    let wrapped = util::wrap_text_en(text, 0);
    assert_eq!(wrapped, "Hello world");
}

#[test]
fn smoke_wrap_line_en() {
    let text = "Hello world this is a test";
    let wrapped = util::wrap_line_en(text, 20);
    assert!(wrapped.contains('\n'));
}

#[test]
fn smoke_count_soft_lines() {
    let text = "Hello world this is a longer text\nAnother line here";
    let count = util::count_soft_lines(text, 20);
    assert!(count >= 2);
}

#[test]
fn smoke_count_soft_lines_empty() {
    assert_eq!(util::count_soft_lines("", 80), 0);
}

#[test]
fn smoke_count_paragraphs() {
    let text = "Paragraph one.\nStill paragraph one.\n\nParagraph two.\n\nParagraph three.";
    assert_eq!(util::count_paragraphs(text), 3);
}

#[test]
fn smoke_count_paragraphs_empty() {
    assert_eq!(util::count_paragraphs(""), 0);
}

#[test]
fn smoke_count_paragraphs_single() {
    assert_eq!(util::count_paragraphs("Single paragraph"), 1);
}

#[test]
fn smoke_count_paragraphs_with_blank_lines() {
    let text = "A\n\n\nB";
    assert_eq!(util::count_paragraphs(text), 2);
}

// ============================================================================
// 10. reader::readtext — FileReader
// ============================================================================

#[test]
fn smoke_file_reader_basic() {
    let path = create_temp_file("test_basic.txt", "line1\nline2\nline3\nline4\nline5\n");
    let reader = reader::readtext::FileReader::new(&path).unwrap();

    assert_eq!(reader.total_lines(), 5);
    assert_eq!(reader.current_line_number(), 0);
}

#[test]
fn smoke_file_reader_seek() {
    let path = create_temp_file("test_seek.txt", "line1\nline2\nline3\nline4\nline5\n");
    let mut reader = reader::readtext::FileReader::new(&path).unwrap();

    reader.seek_to_line(2).unwrap();
    assert_eq!(reader.current_line_number(), 2);

    reader.seek_to_line(10).unwrap();
    assert_eq!(reader.current_line_number(), 5);
}

#[test]
fn smoke_file_reader_read_segment() {
    let path = create_temp_file("test_segment.txt", "line1\nline2\nline3\nline4\nline5\n");
    let mut reader = reader::readtext::FileReader::new(&path).unwrap();

    let lines = reader.read_segment(1, 3).unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line2");
    assert_eq!(lines[1], "line3");
    assert_eq!(lines[2], "line4");
    assert_eq!(reader.current_line_number(), 4);
}

#[test]
fn smoke_file_reader_read_all() {
    let path = create_temp_file("test_all.txt", "a\nb\nc\n");
    let mut reader = reader::readtext::FileReader::new(&path).unwrap();
    let total = reader.total_lines();
    let lines = reader.read_segment(0, total).unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines, vec!["a", "b", "c"]);
}

#[test]
fn smoke_file_reader_empty_file() {
    let path = create_temp_file("test_empty.txt", "");
    let reader = reader::readtext::FileReader::new(&path).unwrap();
    assert_eq!(reader.total_lines(), 0);
}

#[test]
fn smoke_file_reader_iterator() {
    let path = create_temp_file("test_iter.txt", "line1\nline2\nline3\n");
    let mut reader = reader::readtext::FileReader::new(&path).unwrap();

    let lines: Vec<String> = (&mut reader).map(|r| r.unwrap()).collect();

    assert_eq!(lines, vec!["line1", "line2", "line3"]);
    assert_eq!(reader.current_line_number(), 3);
}

// ============================================================================
// 11. reader::readdata — Polars 文件读取
// ============================================================================

#[test]
fn smoke_read_csv_to_lazyframe() {
    let path =
        create_temp_file("test_csv.csv", "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n");

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();

    let df = lf.collect().unwrap();
    assert_eq!(df.height(), 3);
    assert_eq!(df.width(), 3);

    let headers = df.get_column_names();
    assert!(headers.iter().any(|h| h.as_str() == "name"));
    assert!(headers.iter().any(|h| h.as_str() == "age"));
    assert!(headers.iter().any(|h| h.as_str() == "score"));
}

#[test]
fn smoke_read_csv_tsv_separator() {
    let path = create_temp_file("test_tsv.tsv", "name\tage\nAlice\t30\nBob\t25\n");

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b'\t'),
        None,
    )
    .unwrap();

    let df = lf.collect().unwrap();
    assert_eq!(df.height(), 2);
    assert_eq!(df.width(), 2);
}

#[test]
fn smoke_read_json_to_lazyframe() {
    let path = create_temp_file(
        "test_json.json",
        r#"{"name":"Alice","age":30}
{"name":"Bob","age":25}
{"name":"Carol","age":35}"#,
    );

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Json,
        None,
        None,
    )
    .unwrap();

    let df = lf.collect().unwrap();
    assert_eq!(df.height(), 3);
}

#[test]
fn smoke_read_to_lazyframe_nonexistent_file_collect_fails() {
    let result = reader::readdata::read_to_lazyframe(
        "nonexistent_file_xyz.xyz",
        FileFormat::Csv,
        Some(b','),
        None,
    );
    let lf = result.unwrap();
    assert!(lf.collect().is_err());
}

// ============================================================================
// 12. util — 列统计与列选择
// ============================================================================

#[test]
fn smoke_compute_column_stats_numeric() {
    let path = create_temp_file(
        "test_stats.csv",
        "name,value\nAlice,10.5\nBob,20.0\nCarol,30.5\nDave,40.0\n",
    );

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();

    let df = lf.collect().unwrap();
    let stats = util::compute_column_stats(&df, 1, "value", true);

    assert_eq!(stats.name, "value");
    assert!(stats.is_numeric);
    assert_eq!(stats.count, 4);
    assert!(stats.mean.is_some());
    assert!(stats.median.is_some());
    assert!(stats.std_dev.is_some());
    assert!(stats.min.is_some());
    assert!(stats.max.is_some());
}

#[test]
fn smoke_compute_column_stats_string() {
    let path = create_temp_file(
        "test_stats_str.csv",
        "name,value\nAlice,hello\nBob,world\nCarol,hello\n",
    );

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();

    let df = lf.collect().unwrap();
    let stats = util::compute_column_stats(&df, 0, "name", true);

    assert_eq!(stats.name, "name");
    assert!(!stats.is_numeric);
    assert_eq!(stats.count, 3);
}

#[test]
fn smoke_parse_col_selection_by_indices() {
    let sel = util::parse_col_selection("0,2,4").unwrap();
    match sel {
        util::ColSelection::Indices(indices) => {
            assert_eq!(indices, vec![0, 2, 4]);
        }
        _ => panic!("expected Indices"),
    }
}

#[test]
fn smoke_parse_col_selection_by_range() {
    let sel = util::parse_col_selection("0:3").unwrap();
    match sel {
        util::ColSelection::Indices(indices) => {
            assert_eq!(indices, vec![0, 1, 2, 3]);
        }
        _ => panic!("expected Indices"),
    }
}

#[test]
fn smoke_parse_col_selection_by_names() {
    let sel = util::parse_col_selection("name,age,score").unwrap();
    match sel {
        util::ColSelection::Names(names) => {
            assert_eq!(names, vec!["name", "age", "score"]);
        }
        _ => panic!("expected Names"),
    }
}

#[test]
fn smoke_parse_col_selection_empty_errors() {
    assert!(util::parse_col_selection("").is_err());
}

#[test]
fn smoke_resolve_col_selection_indices() {
    let path = create_temp_file("test_resolve.csv", "name,age,score\nAlice,30,95\n");

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();
    let df = lf.collect().unwrap();

    let sel = util::ColSelection::Indices(vec![0, 2]);
    let resolved = util::resolve_col_selection(&df, &sel).unwrap();
    assert_eq!(resolved, vec!["name", "score"]);
}

#[test]
fn smoke_resolve_col_selection_names() {
    let path = create_temp_file("test_resolve2.csv", "name,age,score\nAlice,30,95\n");

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();
    let df = lf.collect().unwrap();

    let sel = util::ColSelection::Names(vec!["name".into(), "score".into()]);
    let resolved = util::resolve_col_selection(&df, &sel).unwrap();
    assert_eq!(resolved, vec!["name", "score"]);
}

#[test]
fn smoke_resolve_col_selection_invalid_index() {
    let path = create_temp_file("test_resolve3.csv", "name,age\nAlice,30\n");

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();
    let df = lf.collect().unwrap();

    let sel = util::ColSelection::Indices(vec![5]);
    assert!(util::resolve_col_selection(&df, &sel).is_err());
}

#[test]
fn smoke_resolve_col_selection_invalid_name() {
    let path = create_temp_file("test_resolve4.csv", "name,age\nAlice,30\n");

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();
    let df = lf.collect().unwrap();

    let sel = util::ColSelection::Names(vec!["nonexistent".into()]);
    assert!(util::resolve_col_selection(&df, &sel).is_err());
}

#[test]
fn smoke_transposed_stats() {
    let path = create_temp_file("test_trans.csv", "name,a,b\nrow1,10.0,20.0\nrow2,30.0,40.0\n");

    let lf = reader::readdata::read_to_lazyframe(
        path.to_str().unwrap(),
        FileFormat::Csv,
        Some(b','),
        None,
    )
    .unwrap();
    let df = lf.collect().unwrap();

    let stats = util::transposed_stats(&df, true, false);
    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].name, "\"row1\"");
}

// ============================================================================
// 13. head 命令 — 统一入口
// ============================================================================

#[test]
fn smoke_head_csv_file() {
    let path = create_temp_file(
        "test_head.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\nDave,28,76.0\nEve,32,89.5\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::head::head_command(&input, 3, false, 80, None).unwrap();
}

#[test]
fn smoke_head_csv_file_no_name() {
    let path = create_temp_file("test_head_noname.csv", "name,age\nAlice,30\nBob,25\nCarol,35\n");
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::head::head_command(&input, 2, true, 80, None).unwrap();
}

#[test]
fn smoke_head_csv_file_with_col_selection() {
    let path = create_temp_file("test_head_col.csv", "name,age,score\nAlice,30,95\nBob,25,88\n");
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::head::head_command(&input, 2, false, 80, Some("name,score")).unwrap();
}

#[test]
fn smoke_head_csv_file_force_csv_on_unknown_ext() {
    let path = create_temp_file(
        "test_head_force.dat",
        "col1,col2\nval1,val2\nval3,val4\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::head::head_command(&input, 2, false, 80, None).unwrap();
}

#[test]
fn smoke_head_csv_from_bytes() {
    let data = b"name,age\nAlice,30\nBob,25\nCarol,35\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::head::head_command(&input, 2, false, 80, None).unwrap();
}

#[test]
fn smoke_head_csv_from_bytes_no_header() {
    let data = b"Alice,30\nBob,25\nCarol,35\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: false,
        },
    );
    simreader::commands::head::head_command(&input, 2, false, 80, None).unwrap();
}

#[test]
fn smoke_head_text_file() {
    let path = create_temp_file(
        "test_head_text.txt",
        "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n",
    );
    let input = file_input(&path, DataFormat::Text);
    simreader::commands::head::head_command(&input, 3, false, 80, None).unwrap();
}

#[test]
fn smoke_head_text_from_bytes() {
    let data = b"A\nB\nC\nD\nE\n".to_vec();
    let input = bytes_input(data, DataFormat::Text);
    simreader::commands::head::head_command(&input, 3, false, 80, None).unwrap();
}

#[test]
fn smoke_head_text_n_larger_than_total() {
    let data = b"line1\nline2\n".to_vec();
    let input = bytes_input(data, DataFormat::Text);
    simreader::commands::head::head_command(&input, 10, false, 80, None).unwrap();
}

// ============================================================================
// 14. tail 命令 — 统一入口
// ============================================================================

#[test]
fn smoke_tail_csv_file() {
    let path = create_temp_file(
        "test_tail.csv",
        "name,age\nAlice,30\nBob,25\nCarol,35\nDave,28\nEve,32\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::tail::tail_command(&input, 2, false, 80, None).unwrap();
}

#[test]
fn smoke_tail_csv_file_more_than_total() {
    let path = create_temp_file("test_tail_overflow.csv", "name,age\nAlice,30\nBob,25\n");
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::tail::tail_command(&input, 10, false, 80, None).unwrap();
}

#[test]
fn smoke_tail_csv_from_bytes() {
    let data = b"name,age\nAlice,30\nBob,25\nCarol,35\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::tail::tail_command(&input, 2, false, 80, None).unwrap();
}

#[test]
fn smoke_tail_text_file() {
    let path = create_temp_file("test_tail_text.txt", "A\nB\nC\nD\nE\n");
    let input = file_input(&path, DataFormat::Text);
    simreader::commands::tail::tail_command(&input, 2, false, 80, None).unwrap();
}

#[test]
fn smoke_tail_text_file_more_than_total() {
    let path = create_temp_file("test_tail_text_overflow.txt", "X\nY\n");
    let input = file_input(&path, DataFormat::Text);
    simreader::commands::tail::tail_command(&input, 10, false, 80, None).unwrap();
}

#[test]
fn smoke_tail_text_from_bytes() {
    let data = b"L1\nL2\nL3\nL4\n".to_vec();
    let input = bytes_input(data, DataFormat::Text);
    simreader::commands::tail::tail_command(&input, 2, false, 80, None).unwrap();
}

// ============================================================================
// 15. schema 命令 — 统一入口
// ============================================================================

#[test]
fn smoke_schema_csv_file_col() {
    let path = create_temp_file(
        "test_schema.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::schema::schema_command(&input, "col", false, None).unwrap();
}

#[test]
fn smoke_schema_csv_file_row() {
    let path = create_temp_file(
        "test_schema_row.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::schema::schema_command(&input, "row", false, None).unwrap();
}

#[test]
fn smoke_schema_csv_from_bytes() {
    let data = b"col1,col2\nval1,val2\nval3,val4\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::schema::schema_command(&input, "col", false, None).unwrap();
}

#[test]
fn smoke_schema_text_file() {
    let path = create_temp_file(
        "test_schema_text.txt",
        "Hello world this is a test file.\nIt has multiple lines.\n中文字符也支持。\n",
    );
    let input = file_input(&path, DataFormat::Text);
    simreader::commands::schema::schema_command(&input, "col", false, None).unwrap();
}

#[test]
fn smoke_schema_text_from_bytes() {
    let data = b"Hello world\nThis is a test\n".to_vec();
    let input = bytes_input(data, DataFormat::Text);
    simreader::commands::schema::schema_command(&input, "col", false, None).unwrap();
}

// ============================================================================
// 16. summary 命令 — 统一入口
// ============================================================================

#[test]
fn smoke_summary_csv_file() {
    let path = create_temp_file(
        "test_summary.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::summary::summary_command(&input, false, None).unwrap();
}

#[test]
fn smoke_summary_csv_file_with_col_selection() {
    let path = create_temp_file(
        "test_summary_col.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::summary::summary_command(&input, false, Some("age,score")).unwrap();
}

#[test]
fn smoke_summary_csv_from_bytes() {
    let data = b"x,y\n1,2\n3,4\n5,6\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    simreader::commands::summary::summary_command(&input, false, None).unwrap();
}

// ============================================================================
// 17. rows 命令 — parse_rows
// ============================================================================

#[test]
fn smoke_parse_rows_specific() {
    let sel = parse_rows("2,5,9").unwrap();
    match sel {
        RowSelection::Specific(nums) => assert_eq!(nums, vec![2, 5, 9]),
        _ => panic!("expected Specific"),
    }
}

#[test]
fn smoke_parse_rows_range() {
    let sel = parse_rows("3:7").unwrap();
    match sel {
        RowSelection::Range(start, end) => {
            assert_eq!(start, 3);
            assert_eq!(end, 7);
        }
        _ => panic!("expected Range"),
    }
}

#[test]
fn smoke_parse_rows_empty_errors() {
    assert!(parse_rows("").is_err());
}

#[test]
fn smoke_parse_rows_range_start_gt_end_errors() {
    assert!(parse_rows("5:2").is_err());
}

#[test]
fn smoke_parse_rows_invalid_format_errors() {
    assert!(parse_rows("1:2:3").is_err());
    assert!(parse_rows("abc").is_err());
}

// ============================================================================
// 18. rows 命令 — 统一入口
// ============================================================================

#[test]
fn smoke_rows_text_file() {
    let path = create_temp_file(
        "test_rows.txt",
        "L0\nL1\nL2\nL3\nL4\nL5\n",
    );
    let input = file_input(&path, DataFormat::Text);
    let sel = RowSelection::Specific(vec![1, 3, 5]);
    simreader::commands::rows::rows_command(&input, &sel, false, 80, false, None).unwrap();
}

#[test]
fn smoke_rows_text_range() {
    let path = create_temp_file(
        "test_rows_range.txt",
        "L0\nL1\nL2\nL3\nL4\n",
    );
    let input = file_input(&path, DataFormat::Text);
    let sel = RowSelection::Range(1, 3);
    simreader::commands::rows::rows_command(&input, &sel, false, 80, false, None).unwrap();
}

#[test]
fn smoke_rows_text_from_bytes() {
    let data = b"row0\nrow1\nrow2\nrow3\n".to_vec();
    let input = bytes_input(data, DataFormat::Text);
    let sel = RowSelection::Specific(vec![0, 2]);
    simreader::commands::rows::rows_command(&input, &sel, false, 80, false, None).unwrap();
}

#[test]
fn smoke_rows_csv_file() {
    let path = create_temp_file(
        "test_rows_csv.csv",
        "name,age\nAlice,30\nBob,25\nCarol,35\n",
    );
    let input = file_input(
        &path,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    let sel = RowSelection::Specific(vec![0, 2]);
    simreader::commands::rows::rows_command(&input, &sel, false, 80, false, None).unwrap();
}

#[test]
fn smoke_rows_csv_from_bytes() {
    let data = b"col1,col2\nval1,val2\nval3,val4\nval5,val6\n".to_vec();
    let input = bytes_input(
        data,
        DataFormat::Csv {
            delimiter: b',',
            has_header: true,
        },
    );
    let sel = RowSelection::Range(0, 1);
    simreader::commands::rows::rows_command(&input, &sel, false, 80, false, None).unwrap();
}

// ============================================================================
// 19. DataFormat ↔ reader::FileFormat 桥接
// ============================================================================

#[test]
fn smoke_input_to_file_format_csv() {
    let input = bytes_input(
        b"".to_vec(),
        DataFormat::Csv {
            delimiter: b'|',
            has_header: true,
        },
    );
    let (fmt, sep) = util::input_to_file_format(&input);
    assert!(matches!(fmt, FileFormat::Csv));
    assert_eq!(sep, Some(b'|'));
}

#[test]
fn smoke_input_to_file_format_json() {
    let input = bytes_input(b"".to_vec(), DataFormat::Json);
    let (fmt, sep) = util::input_to_file_format(&input);
    assert!(matches!(fmt, FileFormat::Json));
    assert_eq!(sep, None);
}

#[test]
fn smoke_input_to_file_format_ipc() {
    let input = bytes_input(b"".to_vec(), DataFormat::Ipc);
    let (fmt, _) = util::input_to_file_format(&input);
    assert!(matches!(fmt, FileFormat::Ipc));
}

#[test]
fn smoke_input_to_file_format_parquet() {
    let input = bytes_input(b"".to_vec(), DataFormat::Parquet);
    let (fmt, _) = util::input_to_file_format(&input);
    assert!(matches!(fmt, FileFormat::Parquet));
}

#[test]
fn smoke_input_to_file_format_excel() {
    let input = bytes_input(b"".to_vec(), DataFormat::Excel);
    let (fmt, _) = util::input_to_file_format(&input);
    assert!(matches!(fmt, FileFormat::Excel));
}
