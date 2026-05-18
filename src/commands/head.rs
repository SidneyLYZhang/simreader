use crate::commands::util;
use crate::input::{DataFormat, InputConfig};

/// 统一入口：根据 `InputConfig` 分派到文本/CSV-流/数据文件处理
pub fn head_command(
    input: &InputConfig,
    n: usize,
    no_name: bool,
    line_width: usize,
    col_selection: Option<&str>,
) -> anyhow::Result<()> {
    match input.format() {
        DataFormat::Text => head_text(input, n, line_width),
        DataFormat::Csv { .. } if input.file_path().is_some() => {
            head_data_file(input, n, no_name, line_width, col_selection)
        }
        DataFormat::Csv { .. } => head_csv_stream(input, n, no_name, line_width, col_selection),
        _ => head_data_file(input, n, no_name, line_width, col_selection),
    }
}

fn head_data_file(
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
    let df = lf.limit(n as u32).collect()?;

    let df = if let Some(col_str) = col_selection {
        let col_sel = util::parse_col_selection(col_str)?;
        let selected = util::resolve_col_selection(&df, &col_sel)?;
        let col_refs: Vec<&str> = selected.iter().map(|s| s.as_str()).collect();
        df.select(col_refs)?
    } else {
        df
    };

    let headers = df.get_column_names();
    let str_headers: Vec<String> = if no_name {
        (0..headers.len()).map(|i| format!("{}", i)).collect()
    } else {
        headers.iter().map(|s| s.to_string()).collect()
    };

    let col_widths: Vec<usize> = str_headers.iter().map(|h| h.chars().count().max(10)).collect();

    println!("{}", str_headers.join("\t"));

    for row_idx in 0..df.height() {
        let row = df.get_row(row_idx)?;
        let cells: Vec<String> = row.0.iter().map(|v| v.to_string()).collect();
        let display: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let char_count = c.chars().count();
                if char_count > col_widths[i] {
                    let truncated: String =
                        c.chars().take(col_widths[i].saturating_sub(3)).collect();
                    format!("{}...", truncated)
                } else {
                    c.clone()
                }
            })
            .collect();
        println!("{}", display.join("\t"));
    }

    Ok(())
}

fn head_text(input: &InputConfig, n: usize, line_width: usize) -> anyhow::Result<()> {
    let reader = input.text_reader()?;
    for line_result in reader.take(n) {
        let line = line_result?;
        let wrapped = util::wrap_line_en(&line, line_width);
        println!("{}", wrapped);
    }
    Ok(())
}

fn head_csv_stream(
    input: &InputConfig,
    n: usize,
    no_name: bool,
    _line_width: usize,
    _col_selection: Option<&str>,
) -> anyhow::Result<()> {
    let reader = input.csv_reader()?;
    let records: Vec<Vec<String>> = reader
        .take(n)
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
    } else if data_start == 0 {
        &records
    } else {
        &[]
    };

    let col_widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let data_max = data_rows
                .iter()
                .map(|r| r.get(i).map(|c| c.chars().count()).unwrap_or(0))
                .max()
                .unwrap_or(0);
            h.chars().count().max(data_max).max(10)
        })
        .collect();

    println!("{}", headers.join("\t"));

    for row in data_rows {
        let display: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let char_count = c.chars().count();
                if char_count > col_widths[i] {
                    let truncated: String =
                        c.chars().take(col_widths[i].saturating_sub(3)).collect();
                    format!("{}...", truncated)
                } else {
                    c.clone()
                }
            })
            .collect();
        println!("{}", display.join("\t"));
    }

    Ok(())
}

