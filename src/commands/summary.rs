use std::path::Path;

use crate::commands::util::{self, compute_column_stats, ColumnStats, WordCount};
use crate::config::ConfigManager;
use crate::reader::readdata::FileFormat;

pub fn summary_data_file(file_path: &str, no_name: bool, force_csv: bool, csv_separator: Option<u8>, col_selection: Option<&str>) -> anyhow::Result<()> {
    let format = if force_csv {
        FileFormat::Csv
    } else {
        util::detect_file_format(file_path)
            .ok_or_else(|| anyhow::anyhow!("不支持的文件格式: {}", file_path))?
    };
    let sep = if force_csv {
        Some(csv_separator.unwrap_or(b','))
    } else {
        util::csv_separator_for_file(file_path)
    };

    let lf = crate::reader::readdata::read_to_lazyframe(file_path, format, sep, None)?;
    let df = lf.collect()?;

    let df = if let Some(col_str) = col_selection {
        let col_sel = util::parse_col_selection(col_str)?;
        let selected = util::resolve_col_selection(&df, &col_sel)?;
        let col_refs: Vec<&str> = selected.iter().map(|s| s.as_str()).collect();
        df.select(col_refs)?
    } else {
        df
    };

    let n_rows = df.height();
    let n_cols = df.width();

    println!("=== 数据文件详细总结 ===");
    println!("数据规模: {} 行 {} 列", n_rows, n_cols);
    println!();

    let headers = df.get_column_names();

    let mut all_stats: Vec<ColumnStats> = Vec::new();
    for (i, header) in headers.iter().enumerate() {
        let name = if no_name {
            format!("{}", i)
        } else {
            header.to_string()
        };

        let stats = compute_column_stats(&df, i, &name, true);
        all_stats.push(stats);
    }

    let numeric_stats: Vec<&ColumnStats> = all_stats.iter().filter(|s| s.is_numeric).collect();
    let string_stats: Vec<&ColumnStats> = all_stats.iter().filter(|s| !s.is_numeric).collect();

    if !numeric_stats.is_empty() {
        print_numeric_summary_table(&numeric_stats);
    }

    if !string_stats.is_empty() {
        print_string_summary_section(&string_stats);
    }

    Ok(())
}

fn print_numeric_summary_table(stats: &[&ColumnStats]) {
    println!("--- 数值列 ---");

    let label_width = 10usize;

    let col_widths: Vec<usize> = stats.iter().map(|s| {
        s.name.chars().count()
            .max(format!("{:.6}", s.mean.unwrap_or(0.0)).len())
            .max(format!("{}", s.count).len())
            .max(s.dtype.len())
            .max(6)
    }).collect();

    let mut fmt_header = format!("{:<label_w$}", "", label_w = label_width);
    for (i, s) in stats.iter().enumerate() {
        fmt_header.push_str(&format!("  {:>col_w$}", s.name, col_w = col_widths[i]));
    }
    println!("{}", fmt_header);

    let sep_line = "─".repeat(fmt_header.chars().count());
    println!("{}", sep_line);

    let fmt_opt = |v: Option<f64>| -> String {
        match v {
            Some(x) => format!("{:.6}", x),
            None => "-".to_string(),
        }
    };

    let fmt_opt_str = |v: Option<&String>| -> String {
        match v {
            Some(x) => x.clone(),
            None => "-".to_string(),
        }
    };

    let rows: Vec<(&str, Vec<String>)> = vec![
        ("类型", stats.iter().map(|s| s.dtype.clone()).collect()),
        ("总数据量", stats.iter().map(|s| format!("{}", s.count)).collect()),
        ("有效数据", stats.iter().map(|s| format!("{}", s.non_zero_valid_count)).collect()),
        ("零值数量", stats.iter().map(|s| format!("{}", s.zero_count)).collect()),
        ("None数量", stats.iter().map(|s| format!("{}", s.none_count)).collect()),
        ("NA数量", stats.iter().map(|s| format!("{}", s.na_count)).collect()),
        ("均值", stats.iter().map(|s| fmt_opt(s.mean)).collect()),
        ("中位数", stats.iter().map(|s| fmt_opt(s.median)).collect()),
        ("众数", stats.iter().map(|s| fmt_opt_str(s.mode.as_ref())).collect()),
        ("标准差", stats.iter().map(|s| fmt_opt(s.std_dev)).collect()),
        ("方差", stats.iter().map(|s| fmt_opt(s.variance)).collect()),
        ("偏度", stats.iter().map(|s| fmt_opt(s.skewness)).collect()),
        ("峰度", stats.iter().map(|s| fmt_opt(s.kurtosis)).collect()),
        ("最小值", stats.iter().map(|s| fmt_opt(s.min)).collect()),
        ("最大值", stats.iter().map(|s| fmt_opt(s.max)).collect()),
    ];

    for (label, values) in &rows {
        print!("{:<label_w$}", label, label_w = label_width);
        for (i, v) in values.iter().enumerate() {
            print!("  {:>col_w$}", v, col_w = col_widths[i]);
        }
        println!();
    }
    println!();
}

fn print_string_summary_section(stats: &[&ColumnStats]) {
    println!("--- 字符串列 ---");
    println!();

    for s in stats {
        println!("  列: {}", s.name);
        println!("  数据类型: {}", s.dtype);
        println!("  总数量: {}", s.count);
        if let Some(ref wc) = s.word_count {
            println!("  英文词数: {}", wc.en_words);
            println!("  中文字数: {}", wc.cn_chars);
            println!("  总词数: {}", wc.total);
        }
        if let Some(ref word) = s.top_freq_word {
            println!("  最高词频单词: {}", word);
        }
        if let Some(ref top) = s.top_freq_content {
            println!("  重复最多次的内容: \"{}\" (出现 {} 次)", top.0, top.1);
        }
        if let Some(ref longest) = s.longest {
            println!("  最长内容: \"{}\"", longest);
        }
        if let Some(ref shortest) = s.shortest {
            println!("  最短内容: \"{}\"", shortest);
        }
        println!();
    }
}

pub async fn summary_text_file(file_path: &str) -> anyhow::Result<()> {
    let mgr = ConfigManager::new()?;
    let line_width = mgr.line_width();

    let path = Path::new(file_path);
    let reader = crate::reader::readtext::FileReader::new(path)?;
    let total_lines = reader.total_lines();

    let mut reader = reader;
    let all_lines = reader.read_segment(0, total_lines)?;
    let full_text = all_lines.join("\n");

    let wc = WordCount::from_text(&full_text);
    let soft_lines = util::count_soft_lines(&full_text, line_width);
    let paragraphs = util::count_paragraphs(&full_text);

    println!("=== 文本文件总结 ===");
    println!("英文词数: {} (去标点: {})", wc.en_words,
        util::en_words_only(&util::clean_punct(&full_text)));
    println!("中文字数: {} (去标点: {})", wc.cn_chars,
        util::cn_chars_only(&util::clean_punct(&full_text)));
    println!("行数(按{}字符宽度软换行): {}", line_width, soft_lines);
    println!("段落数: {}", paragraphs);

    let llm_configured = check_llm_configured(&mgr);
    if !llm_configured {
        return Ok(());
    }

    let api_key = match mgr.get_api_key_for_current_provider() {
        Ok(key) => key,
        Err(_) => {
            return Ok(());
        }
    };

    let provider = create_llm_provider(&mgr, &api_key);
    match provider {
        Ok(provider) => {
            let paragraphs_text: Vec<&str> = full_text.split("\n\n")
                .filter(|p| !p.trim().is_empty())
                .collect();

            if !paragraphs_text.is_empty() {
                println!();
                println!("--- 段落大意 (LLM) ---");
                for (i, para) in paragraphs_text.iter().enumerate() {
                    let para_clean: String = para.lines()
                        .map(|l| l.trim())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if para_clean.len() > 2000 {
                        let truncated = para_clean.chars().take(2000).collect::<String>();
                        match summarize_paragraph(&*provider, &truncated, &mgr).await {
                            Ok(summary) => println!("  段落 {}: {}", i + 1, summary),
                            Err(_) => println!("  段落 {}: (LLM 调用失败)", i + 1),
                        }
                    } else {
                        match summarize_paragraph(&*provider, &para_clean, &mgr).await {
                            Ok(summary) => println!("  段落 {}: {}", i + 1, summary),
                            Err(_) => println!("  段落 {}: (LLM 调用失败)", i + 1),
                        }
                    }
                }
            }

            let overview = full_text.chars().take(3000).collect::<String>();
            println!();
            println!("--- 全文概述 (LLM) ---");
            match summarize_overall(&*provider, &overview, &mgr).await {
                Ok(summary) => println!("{}", summary),
                Err(_) => println!("(LLM 调用失败)"),
            }
        }
        Err(_) => {}
    }

    Ok(())
}

fn check_llm_configured(mgr: &ConfigManager) -> bool {
    let cfg = mgr.config();
    !cfg.llm.provider.is_empty()
}

pub fn create_llm_provider(
    mgr: &ConfigManager,
    api_key: &str,
) -> anyhow::Result<Box<dyn crate::llm::LlmProvider>> {
    let cfg = mgr.config();
    let provider_name = cfg.llm.provider.to_lowercase();

    match provider_name.as_str() {
        "deepseek" => {
            let p = crate::llm::deepseek::DeepSeekProvider::new(api_key)?
                .with_model(&cfg.llm.model)
                .with_base_url(&cfg.llm.base_url);
            Ok(Box::new(p))
        }
        "openrouter" => {
            let p = crate::llm::openrouter::OpenRouterProvider::new(api_key)?
                .with_model(&cfg.llm.model)
                .with_base_url(&cfg.llm.base_url);
            Ok(Box::new(p))
        }
        _ => {
            anyhow::bail!("不支持的 LLM 供应商: {}", cfg.llm.provider)
        }
    }
}

async fn summarize_paragraph(
    provider: &dyn crate::llm::LlmProvider,
    text: &str,
    mgr: &ConfigManager,
) -> anyhow::Result<String> {
    let lang = mgr.output_language();

    let request = crate::llm::ChatRequest {
        model: mgr.config().llm.model.clone(),
        messages: vec![
            crate::llm::ChatMessage::system(&format!(
                "\
请用不超过30个词/字（不含标点）总结以下段落的大意。
仅输出最终总结，不要包含任何思考过程、自我反思、元评论或格式说明。
请用{}输出。",
                lang
            )),
            crate::llm::ChatMessage::user(text),
        ],
        temperature: Some(0.3),
        max_tokens: Some(500),
        top_p: None,
        thinking: None,
        files: vec![],
    };

    let response = provider.chat(request).await?;
    let summary = response.content.trim().to_string();
    if summary.is_empty() {
        anyhow::bail!("LLM 返回了空内容");
    }
    Ok(summary)
}

async fn summarize_overall(
    provider: &dyn crate::llm::LlmProvider,
    text: &str,
    mgr: &ConfigManager,
) -> anyhow::Result<String> {
    let lang = mgr.output_language();

    let request = crate::llm::ChatRequest {
        model: mgr.config().llm.model.clone(),
        messages: vec![
            crate::llm::ChatMessage::system(&format!(
                "\
请对以下文字进行全面概述。要求：
1. 长度至少 100 个英文单词或 300 个中文字符（不含标点）。
2. 表达简洁精炼，避免冗余，突出重点信息。
3. 仅输出最终概述正文，不要包含任何思考过程、自我反思、元评论或格式说明。
4. 请用{}输出。",
                lang
            )),
            crate::llm::ChatMessage::user(text),
        ],
        temperature: Some(0.3),
        max_tokens: Some(1500),
        top_p: None,
        thinking: None,
        files: vec![],
    };

    let response = provider.chat(request).await?;
    let summary = response.content.trim().to_string();
    if summary.is_empty() {
        anyhow::bail!("LLM 返回了空内容");
    }
    Ok(summary)
}
