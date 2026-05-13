use simreader::config::{AppConfig, ConfigManager, ReasoningEffort};
use simreader::commands::util;
use simreader::llm;
use simreader::reader;
use simreader::reader::readdata::FileFormat;

use std::io::Write;

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

fn create_temp_file(name: &str, content: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("simreader_smoke_test_files");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

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

    let lines: Vec<String> = (&mut reader)
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(lines, vec!["line1", "line2", "line3"]);
    assert_eq!(reader.current_line_number(), 3);
}

#[test]
fn smoke_read_csv_to_lazyframe() {
    let path = create_temp_file("test_csv.csv", "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n");

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
    let path = create_temp_file(
        "test_resolve.csv",
        "name,age,score\nAlice,30,95\n",
    );

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
    let path = create_temp_file(
        "test_resolve2.csv",
        "name,age,score\nAlice,30,95\n",
    );

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
    let path = create_temp_file(
        "test_resolve3.csv",
        "name,age\nAlice,30\n",
    );

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
    let path = create_temp_file(
        "test_resolve4.csv",
        "name,age\nAlice,30\n",
    );

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
    let path = create_temp_file(
        "test_trans.csv",
        "name,a,b\nrow1,10.0,20.0\nrow2,30.0,40.0\n",
    );

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

#[test]
fn smoke_file_format_debug() {
    assert_eq!(format!("{:?}", FileFormat::Csv), "Csv");
    assert_eq!(format!("{:?}", FileFormat::Json), "Json");
    assert_eq!(format!("{:?}", FileFormat::Ipc), "Ipc");
    assert_eq!(format!("{:?}", FileFormat::Parquet), "Parquet");
    assert_eq!(format!("{:?}", FileFormat::Excel), "Excel");
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
    assert_eq!(parsed["messages"][0]["content"], "sys");
    assert_eq!(parsed["messages"][1]["role"], "user");
    assert_eq!(parsed["messages"][1]["content"], "query");
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

#[test]
fn smoke_head_data_file() {
    let path = create_temp_file(
        "test_head.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\nDave,28,76.0\nEve,32,89.5\n",
    );

    simreader::commands::head::head_data_file(
        path.to_str().unwrap(),
        3,
        false,
        80,
        false,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn smoke_head_data_file_no_name() {
    let path = create_temp_file(
        "test_head_noname.csv",
        "name,age\nAlice,30\nBob,25\nCarol,35\n",
    );

    simreader::commands::head::head_data_file(
        path.to_str().unwrap(),
        2,
        true,
        80,
        false,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn smoke_head_data_file_with_col_selection() {
    let path = create_temp_file(
        "test_head_col.csv",
        "name,age,score\nAlice,30,95\nBob,25,88\n",
    );

    simreader::commands::head::head_data_file(
        path.to_str().unwrap(),
        2,
        false,
        80,
        false,
        None,
        Some("name,score"),
    )
    .unwrap();
}

#[test]
fn smoke_head_data_file_force_csv() {
    let path = create_temp_file(
        "test_head_force.dat",
        "col1,col2\nval1,val2\nval3,val4\n",
    );

    simreader::commands::head::head_data_file(
        path.to_str().unwrap(),
        2,
        false,
        80,
        true,
        Some(b','),
        None,
    )
    .unwrap();
}

#[test]
fn smoke_head_text_file() {
    let path = create_temp_file(
        "test_head_text.txt",
        "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n",
    );

    simreader::commands::head::head_text_file(path.to_str().unwrap(), 3, 80).unwrap();
}

#[test]
fn smoke_tail_data_file() {
    let path = create_temp_file(
        "test_tail.csv",
        "name,age\nAlice,30\nBob,25\nCarol,35\nDave,28\nEve,32\n",
    );

    simreader::commands::tail::tail_data_file(
        path.to_str().unwrap(),
        2,
        false,
        80,
        false,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn smoke_tail_data_file_more_than_total() {
    let path = create_temp_file(
        "test_tail_overflow.csv",
        "name,age\nAlice,30\nBob,25\n",
    );

    simreader::commands::tail::tail_data_file(
        path.to_str().unwrap(),
        10,
        false,
        80,
        false,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn smoke_tail_text_file() {
    let path = create_temp_file(
        "test_tail_text.txt",
        "A\nB\nC\nD\nE\n",
    );

    simreader::commands::tail::tail_text_file(path.to_str().unwrap(), 2, 80).unwrap();
}

#[test]
fn smoke_tail_text_file_more_than_total() {
    let path = create_temp_file(
        "test_tail_text_overflow.txt",
        "X\nY\n",
    );

    simreader::commands::tail::tail_text_file(path.to_str().unwrap(), 10, 80).unwrap();
}

#[test]
fn smoke_schema_data_file_col_direction() {
    let path = create_temp_file(
        "test_schema.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n",
    );

    simreader::commands::schema::schema_data_file(
        path.to_str().unwrap(),
        "col",
        false,
        false,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn smoke_schema_data_file_row_direction() {
    let path = create_temp_file(
        "test_schema_row.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\n",
    );

    simreader::commands::schema::schema_data_file(
        path.to_str().unwrap(),
        "row",
        false,
        false,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn smoke_schema_text_file() {
    let path = create_temp_file(
        "test_schema_text.txt",
        "Hello world this is a test file.\nIt has multiple lines.\n中文字符也支持。\n",
    );

    simreader::commands::schema::schema_text_file(path.to_str().unwrap()).unwrap();
}

#[test]
fn smoke_summary_data_file() {
    let path = create_temp_file(
        "test_summary.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n",
    );

    simreader::commands::summary::summary_data_file(
        path.to_str().unwrap(),
        false,
        false,
        None,
        None,
    )
    .unwrap();
}

#[test]
fn smoke_summary_data_file_with_col_selection() {
    let path = create_temp_file(
        "test_summary_col.csv",
        "name,age,score\nAlice,30,95.5\nBob,25,88.0\nCarol,35,92.3\n",
    );

    simreader::commands::summary::summary_data_file(
        path.to_str().unwrap(),
        false,
        false,
        None,
        Some("age,score"),
    )
    .unwrap();
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
