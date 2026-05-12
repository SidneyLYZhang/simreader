use std::path::Path;

use crate::commands::util::{self, compute_column_stats, transposed_stats, ColumnStats};
use crate::config::ConfigManager;
use crate::reader::readdata::FileFormat;

pub fn schema_data_file(
    file_path: &str,
    direction: &str,
    no_name: bool,
    force_csv: bool,
    csv_separator: Option<u8>,
    col_selection: Option<&str>,
) -> anyhow::Result<()> {
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

    println!("=== 数据文件 Schema ===");
    println!("数据规模: {} 行 {} 列", n_rows, n_cols);
    println!();

    if direction == "col" {
        let headers = df.get_column_names();

        let mut all_stats: Vec<ColumnStats> = Vec::new();
        for (i, header) in headers.iter().enumerate() {
            let name = if no_name {
                format!("{}", i)
            } else {
                header.to_string()
            };
            let stats = compute_column_stats(&df, i, &name, false);
            all_stats.push(stats);
        }

        let numeric_stats: Vec<&ColumnStats> = all_stats.iter().filter(|s| s.is_numeric).collect();
        let string_stats: Vec<&ColumnStats> = all_stats.iter().filter(|s| !s.is_numeric).collect();

        if !numeric_stats.is_empty() {
            print_numeric_schema_table(&numeric_stats);
        }

        if !string_stats.is_empty() {
            print_string_schema_section(&string_stats);
        }
    } else {
        let all_stats = transposed_stats(&df, !no_name, false);

        let numeric_stats: Vec<&ColumnStats> = all_stats.iter().filter(|s| s.is_numeric).collect();
        let string_stats: Vec<&ColumnStats> = all_stats.iter().filter(|s| !s.is_numeric).collect();

        if !numeric_stats.is_empty() {
            print_numeric_schema_table(&numeric_stats);
        }

        if !string_stats.is_empty() {
            print_string_schema_section(&string_stats);
        }
    }

    Ok(())
}

fn print_numeric_schema_table(stats: &[&ColumnStats]) {
    println!("--- 数值列 ---");

    let name_width = stats.iter().map(|s| s.name.chars().count()).max().unwrap_or(4).max(4);
    let dtype_width = stats.iter().map(|s| s.dtype.len()).max().unwrap_or(4).max(4);
    let count_width = stats.iter().map(|s| format!("{}", s.count).len()).max().unwrap_or(4).max(4);

    let fmt_val = |v: Option<f64>| -> String {
        match v {
            Some(x) => format!("{:.4}", x),
            None => "-".to_string(),
        }
    };

    let mean_width = stats.iter().map(|s| fmt_val(s.mean).len()).max().unwrap_or(4).max(4);
    let median_width = stats.iter().map(|s| fmt_val(s.median).len()).max().unwrap_or(6).max(6);
    let min_width = stats.iter().map(|s| fmt_val(s.min).len()).max().unwrap_or(6).max(6);
    let max_width = stats.iter().map(|s| fmt_val(s.max).len()).max().unwrap_or(6).max(6);

    let header = format!(
        "{:<name_w$}  {:<dtype_w$}  {:>count_w$}  {:>mean_w$}  {:>median_w$}  {:>min_w$}  {:>max_w$}",
        "列名", "类型", "总数", "均值", "中位数", "最小值", "最大值",
        name_w = name_width, dtype_w = dtype_width, count_w = count_width,
        mean_w = mean_width, median_w = median_width, min_w = min_width, max_w = max_width,
    );
    let sep_line = "─".repeat(header.chars().count());
    println!("{}", header);
    println!("{}", sep_line);

    for s in stats {
        println!(
            "{:<name_w$}  {:<dtype_w$}  {:>count_w$}  {:>mean_w$}  {:>median_w$}  {:>min_w$}  {:>max_w$}",
            s.name,
            s.dtype,
            s.count,
            fmt_val(s.mean),
            fmt_val(s.median),
            fmt_val(s.min),
            fmt_val(s.max),
            name_w = name_width, dtype_w = dtype_width, count_w = count_width,
            mean_w = mean_width, median_w = median_width, min_w = min_width, max_w = max_width,
        );
    }
    println!();
}

fn print_string_schema_section(stats: &[&ColumnStats]) {
    println!("--- 字符串列 ---");
    println!();

    for s in stats {
        println!("  列: {}", s.name);
        println!("  数据类型: {}", s.dtype);
        if let Some(ref wc) = s.word_count {
            println!("  英文词数: {}, 中文字数: {}, 总词数: {}", wc.en_words, wc.cn_chars, wc.total);
        }
        if let Some(ref top) = s.top_freq_content {
            println!("  最频繁内容: \"{}\" (出现 {} 次)", top.0, top.1);
        }
        println!();
    }
}

pub fn schema_text_file(file_path: &str) -> anyhow::Result<()> {
    let mgr = ConfigManager::new()?;
    let line_width = mgr.line_width();

    let path = Path::new(file_path);
    let reader = crate::reader::readtext::FileReader::new(path)?;
    let total_lines = reader.total_lines();

    let mut reader = reader;
    let all_lines = reader.read_segment(0, total_lines)?;
    let full_text = all_lines.join("\n");

    let wc = util::WordCount::from_text(&full_text);
    let soft_lines = util::count_soft_lines(&full_text, line_width);
    let paragraphs = util::count_paragraphs(&full_text);

    println!("=== 文本文件 Schema ===");
    println!("英文词数: {}", wc.en_words);
    println!("中文字数: {}", wc.cn_chars);
    println!("总词数: {}", wc.total);
    println!("行数(按{}字符宽度软换行): {}", line_width, soft_lines);
    println!("段落数: {}", paragraphs);

    Ok(())
}
