use std::path::Path;

use crate::commands::util;
use crate::reader::readdata::FileFormat;

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
        let nums = nums.map_err(|_| anyhow::anyhow!("无效的行号列表: {}，行号必须为非负整数", input))?;
        if nums.is_empty() {
            anyhow::bail!("行号列表不能为空");
        }
        Ok(RowSelection::Specific(nums))
    }
}

pub fn rows_text_file(
    file_path: &str,
    selection: &RowSelection,
    line_width: usize,
    txt_mode: bool,
) -> anyhow::Result<()> {
    let path = Path::new(file_path);
    if !path.exists() {
        anyhow::bail!("文件不存在: {}", file_path);
    }
    let mut reader = crate::reader::readtext::FileReader::new(path)?;
    let total = reader.total_lines();

    if total == 0 {
        println!("(文件为空)");
        return Ok(());
    }

    match selection {
        RowSelection::Specific(nums) => {
            for &line_num in nums {
                if line_num >= total {
                    eprintln!("警告: 行号 {} 超出文件范围（总行数: {}，最大有效行号: {}）", line_num, total, total.saturating_sub(1));
                    continue;
                }
                let lines = reader.read_segment(line_num, 1)?;
                if let Some(line) = lines.first() {
                    if txt_mode || line_width == 0 {
                        println!("{}", line);
                    } else {
                        let wrapped = util::wrap_line_en(line, line_width);
                        println!("{}", wrapped);
                    }
                }
            }
        }
        RowSelection::Range(start, end) => {
            let start = *start;
            let end = (*end).min(total.saturating_sub(1));
            if start > end {
                anyhow::bail!("起始行号 {} 超出文件范围（总行数: {}）", start, total);
            }
            if start >= total {
                anyhow::bail!("起始行号 {} 超出文件范围（总行数: {}，最大有效行号: {}）", start, total, total.saturating_sub(1));
            }
            let count = end - start + 1;
            let lines = reader.read_segment(start, count)?;
            for line in &lines {
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

pub fn rows_data_file(
    file_path: &str,
    selection: &RowSelection,
    no_name: bool,
    _line_width: usize,
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
