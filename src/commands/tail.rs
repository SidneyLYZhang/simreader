use std::io;

use crate::commands::util;
use crate::input::{DataFormat, InputConfig};

/// 统一入口：根据 `InputConfig` 分派到文本/CSV-流/数据文件处理
pub fn tail_command(
    input: &InputConfig,
    n: usize,
    no_name: bool,
    line_width: usize,
    col_selection: Option<&str>,
) -> anyhow::Result<()> {
    match input.format() {
        DataFormat::Text => tail_text(input, n, line_width),
        DataFormat::Csv { .. } if input.file_path().is_some() => {
            tail_data_file(input, n, no_name, line_width, col_selection)
        }
        DataFormat::Csv { .. } => tail_csv_stream(input, n, no_name, line_width, col_selection),
        _ => tail_data_file(input, n, no_name, line_width, col_selection),
    }
}

fn tail_data_file(
    input: &InputConfig,
    n: usize,
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
    let df = lf.collect()?;

    let df = if let Some(col_str) = col_selection {
        let col_sel = util::parse_col_selection(col_str)?;
        let selected = util::resolve_col_selection(&df, &col_sel)?;
        let col_refs: Vec<&str> = selected.iter().map(|s| s.as_str()).collect();
        df.select(col_refs)?
    } else {
        df
    };

    let total_rows = df.height();
    let start = if n >= total_rows { 0 } else { total_rows - n };

    let tail_df = df.slice(start as i64, n);

    let headers = df.get_column_names();
    let str_headers: Vec<String> = if no_name {
        (0..headers.len()).map(|i| format!("{}", i)).collect()
    } else {
        headers.iter().map(|s| s.to_string()).collect()
    };

    println!("{}", str_headers.join("\t"));

    for row_idx in 0..tail_df.height() {
        let row = tail_df.get_row(row_idx)?;
        let cells: Vec<String> = row.0.iter().map(|v| v.to_string()).collect();
        println!("{}", cells.join("\t"));
    }

    Ok(())
}

fn tail_text(input: &InputConfig, n: usize, line_width: usize) -> anyhow::Result<()> {
    let reader = input.text_reader()?;
    let all_lines: Vec<String> = reader.collect::<io::Result<Vec<_>>>()?;

    let total = all_lines.len();
    let start = total.saturating_sub(n.min(total));
    for line in &all_lines[start..] {
        let wrapped = util::wrap_line_en(line, line_width);
        println!("{}", wrapped);
    }
    Ok(())
}

fn tail_csv_stream(
    input: &InputConfig,
    n: usize,
    no_name: bool,
    _line_width: usize,
    _col_selection: Option<&str>,
) -> anyhow::Result<()> {
    let reader = input.csv_reader()?;
    let records: Vec<Vec<String>> = reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("CSV 解析错误: {}", e))?;

    if records.is_empty() {
        return Ok(());
    }

    let headers: Vec<String> = if no_name {
        (0..records[0].len()).map(|i| format!("{}", i)).collect()
    } else {
        records[0].clone()
    };

    let data_start: usize = if input.format().has_header() && !no_name { 1 } else { 0 };
    let data_rows: &[Vec<String>] = if data_start > 0 && records.len() > data_start {
        &records[data_start..]
    } else {
        &records
    };

    let total = data_rows.len();
    let start = total.saturating_sub(n.min(total));
    let tail_rows = &data_rows[start..];

    println!("{}", headers.join("\t"));
    for row in tail_rows {
        println!("{}", row.join("\t"));
    }

    Ok(())
}
