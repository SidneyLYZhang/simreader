use crate::commands::util;
use crate::input::{DataFormat, InputConfig};

#[derive(Debug, Clone)]
pub enum RowSelection {
    Specific(Vec<usize>),
    Range(usize, usize),
}

pub fn parse_rows(input: &str) -> anyhow::Result<RowSelection> {
    let input = input.trim();
    if input.is_empty() {
        anyhow::bail!("行号参数不能为空");
    }

    if input.contains(':') {
        let parts: Vec<&str> = input.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("无效的行范围格式: {}，正确格式如 3:7", input);
        }
        let start: usize = parts[0]
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("无效的起始行号: {}", parts[0]))?;
        let end: usize = parts[1]
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("无效的结束行号: {}", parts[1]))?;
        if start > end {
            anyhow::bail!("起始行号 {} 不能大于结束行号 {}", start, end);
        }
        Ok(RowSelection::Range(start, end))
    } else {
        let nums: Result<Vec<usize>, _> = input
            .split(',')
            .map(|s| s.trim().parse::<usize>())
            .collect();
        let nums =
            nums.map_err(|_| anyhow::anyhow!("无效的行号列表: {}，行号必须为非负整数", input))?;
        if nums.is_empty() {
            anyhow::bail!("行号列表不能为空");
        }
        Ok(RowSelection::Specific(nums))
    }
}

/// 统一入口
pub fn rows_command(
    input: &InputConfig,
    selection: &RowSelection,
    no_name: bool,
    line_width: usize,
    txt_mode: bool,
    col_selection: Option<&str>,
) -> anyhow::Result<()> {
    match input.format() {
        DataFormat::Text => rows_text(input, selection, line_width, txt_mode),
        DataFormat::Csv { .. } if input.file_path().is_some() => {
            rows_data_file(input, selection, no_name, line_width, col_selection)
        }
        DataFormat::Csv { .. } => {
            rows_csv_stream(input, selection, no_name, line_width, col_selection)
        }
        _ => rows_data_file(input, selection, no_name, line_width, col_selection),
    }
}

fn rows_text(
    input: &InputConfig,
    selection: &RowSelection,
    line_width: usize,
    txt_mode: bool,
) -> anyhow::Result<()> {
    let reader = input.text_reader()?;
    let all_lines: Vec<String> = reader.collect::<std::io::Result<Vec<_>>>()?;
    let total = all_lines.len();

    if total == 0 {
        println!("(输入为空)");
        return Ok(());
    }

    match selection {
        RowSelection::Specific(nums) => {
            for &line_num in nums {
                if line_num >= total {
                    eprintln!(
                        "警告: 行号 {} 超出范围（总行数: {}，最大有效行号: {}）",
                        line_num,
                        total,
                        total.saturating_sub(1)
                    );
                    continue;
                }
                let line = &all_lines[line_num];
                if txt_mode || line_width == 0 {
                    println!("{}", line);
                } else {
                    let wrapped = util::wrap_line_en(line, line_width);
                    println!("{}", wrapped);
                }
            }
        }
        RowSelection::Range(start, end) => {
            let start = *start;
            let end = (*end).min(total.saturating_sub(1));
            if start > end {
                anyhow::bail!("起始行号 {} 超出范围（总行数: {}）", start, total);
            }
            if start >= total {
                anyhow::bail!(
                    "起始行号 {} 超出范围（总行数: {}，最大有效行号: {}）",
                    start,
                    total,
                    total.saturating_sub(1)
                );
            }
            for line in &all_lines[start..=end] {
                if txt_mode || line_width == 0 {
                    println!("{}", line);
                } else {
                    let wrapped = util::wrap_line_en(line, line_width);
                    println!("{}", wrapped);
                }
            }
        }
    }

    Ok(())
}

fn rows_data_file(
    input: &InputConfig,
    selection: &RowSelection,
    no_name: bool,
    _line_width: usize,
    col_selection: Option<&str>,
) -> anyhow::Result<()> {
    let file_path = input.file_path().unwrap();
    let (format, sep) = util::input_to_file_format(input);

    let lf = crate::reader::readdata::read_to_lazyframe(
        file_path.to_str().unwrap(),
        format,
        sep,
        None,
    )?;
    let df_raw = lf.collect()?;

    let df_raw = if let Some(col_str) = col_selection {
        let col_sel = util::parse_col_selection(col_str)?;
        let selected = util::resolve_col_selection(&df_raw, &col_sel)?;
        let col_refs: Vec<&str> = selected.iter().map(|s| s.as_str()).collect();
        df_raw.select(col_refs)?
    } else {
        df_raw
    };

    let total_rows = df_raw.height();

    if total_rows == 0 {
        println!("(数据文件为空)");
        return Ok(());
    }

    let headers = df_raw.get_column_names();
    let str_headers: Vec<String> = if no_name {
        (0..headers.len()).map(|i| format!("{}", i)).collect()
    } else {
        headers.iter().map(|s| s.to_string()).collect()
    };

    let mut extended_headers = vec!["Row".to_string()];
    extended_headers.extend(str_headers.clone());

    let col_widths: Vec<usize> = extended_headers
        .iter()
        .map(|h| h.chars().count().max(10))
        .collect();

    println!("{}", extended_headers.join("\t"));

    match selection {
        RowSelection::Specific(nums) => {
            for &row_idx in nums {
                if row_idx >= total_rows {
                    eprintln!(
                        "警告: 行号 {} 超出范围（总行数: {}，最大有效行号: {}）",
                        row_idx,
                        total_rows,
                        total_rows.saturating_sub(1)
                    );
                    continue;
                }
                let row = df_raw.get_row(row_idx)?;
                let cells: Vec<String> = row.0.iter().map(|v| v.to_string()).collect();

                let mut display: Vec<String> = vec![row_idx.to_string()];
                display.extend(cells.iter().enumerate().map(|(i, c)| {
                    let char_count = c.chars().count();
                    if char_count > col_widths[i + 1] {
                        let truncated: String =
                            c.chars().take(col_widths[i + 1].saturating_sub(3)).collect();
                        format!("{}...", truncated)
                    } else {
                        c.clone()
                    }
                }));
                println!("{}", display.join("\t"));
            }
        }
        RowSelection::Range(start, end) => {
            let start = *start;
            let end = (*end).min(total_rows.saturating_sub(1));
            if start > end {
                anyhow::bail!("起始行号 {} 超出数据范围（总行数: {}）", start, total_rows);
            }
            if start >= total_rows {
                anyhow::bail!(
                    "起始行号 {} 超出数据范围（总行数: {}，最大有效行号: {}）",
                    start,
                    total_rows,
                    total_rows.saturating_sub(1)
                );
            }
            let count = end - start + 1;
            let sliced = df_raw.slice(start as i64, count);

            for row_rel_idx in 0..sliced.height() {
                let actual_row = start + row_rel_idx;
                let row = sliced.get_row(row_rel_idx)?;
                let cells: Vec<String> = row.0.iter().map(|v| v.to_string()).collect();

                let mut display: Vec<String> = vec![actual_row.to_string()];
                display.extend(cells.iter().enumerate().map(|(i, c)| {
                    let char_count = c.chars().count();
                    if char_count > col_widths[i + 1] {
                        let truncated: String =
                            c.chars().take(col_widths[i + 1].saturating_sub(3)).collect();
                        format!("{}...", truncated)
                    } else {
                        c.clone()
                    }
                }));
                println!("{}", display.join("\t"));
            }
        }
    }

    Ok(())
}

fn rows_csv_stream(
    input: &InputConfig,
    selection: &RowSelection,
    no_name: bool,
    _line_width: usize,
    _col_selection: Option<&str>,
) -> anyhow::Result<()> {
    let reader = input.csv_reader()?;
    let records: Vec<Vec<String>> = reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("CSV 解析错误: {}", e))?;

    if records.is_empty() {
        println!("(CSV 数据为空)");
        return Ok(());
    }

    let has_header = input.format().has_header();
    let headers: Vec<String> = if has_header {
        records[0].clone()
    } else {
        (0..records[0].len())
            .map(|i| format!("column_{}", i))
            .collect()
    };
    let data_start: usize = if has_header { 1 } else { 0 };
    let data_rows = &records[data_start..];

    let total_rows = data_rows.len();

    if total_rows == 0 {
        println!("(CSV 数据为空)");
        return Ok(());
    }

    let str_headers: Vec<String> = if no_name {
        (0..headers.len()).map(|i| format!("{}", i)).collect()
    } else {
        headers.clone()
    };

    let mut extended_headers = vec!["Row".to_string()];
    extended_headers.extend(str_headers.clone());

    let col_widths: Vec<usize> = extended_headers
        .iter()
        .map(|h| h.chars().count().max(10))
        .collect();

    println!("{}", extended_headers.join("\t"));

    match selection {
        RowSelection::Specific(nums) => {
            for &row_idx in nums {
                if row_idx >= total_rows {
                    eprintln!(
                        "警告: 行号 {} 超出范围（总行数: {}，最大有效行号: {}）",
                        row_idx,
                        total_rows,
                        total_rows.saturating_sub(1)
                    );
                    continue;
                }
                let row = &data_rows[row_idx];
                let mut display: Vec<String> = vec![row_idx.to_string()];
                display.extend(row.iter().enumerate().map(|(i, c)| {
                    let char_count = c.chars().count();
                    if char_count > col_widths[i + 1] {
                        let truncated: String =
                            c.chars().take(col_widths[i + 1].saturating_sub(3)).collect();
                        format!("{}...", truncated)
                    } else {
                        c.clone()
                    }
                }));
                println!("{}", display.join("\t"));
            }
        }
        RowSelection::Range(start, end) => {
            let start = *start;
            let end = (*end).min(total_rows.saturating_sub(1));
            if start > end {
                anyhow::bail!("起始行号 {} 超出数据范围（总行数: {}）", start, total_rows);
            }
            if start >= total_rows {
                anyhow::bail!(
                    "起始行号 {} 超出数据范围（总行数: {}，最大有效行号: {}）",
                    start,
                    total_rows,
                    total_rows.saturating_sub(1)
                );
            }
            for (offset, row) in data_rows[start..=end].iter().enumerate() {
                let actual_row = start + offset;
                let mut display: Vec<String> = vec![actual_row.to_string()];
                display.extend(row.iter().enumerate().map(|(i, c)| {
                    let char_count = c.chars().count();
                    if char_count > col_widths[i + 1] {
                        let truncated: String =
                            c.chars().take(col_widths[i + 1].saturating_sub(3)).collect();
                        format!("{}...", truncated)
                    } else {
                        c.clone()
                    }
                }));
                println!("{}", display.join("\t"));
            }
        }
    }

    Ok(())
}
