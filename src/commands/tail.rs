use std::path::Path;

use crate::commands::util;
use crate::reader::readdata::FileFormat;

pub fn tail_data_file(file_path: &str, n: usize, no_name: bool, _line_width: usize, force_csv: bool, csv_separator: Option<u8>, col_selection: Option<&str>) -> anyhow::Result<()> {
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

pub fn tail_text_file(file_path: &str, n: usize, line_width: usize) -> anyhow::Result<()> {
    let path = Path::new(file_path);
    let mut reader = crate::reader::readtext::FileReader::new(path)?;
    let total = reader.total_lines();
    let count = n.min(total);
    let start = total.saturating_sub(count);
    let lines = reader.read_segment(start, count)?;

    for line in &lines {
        let wrapped = util::wrap_line_en(line, line_width);
        println!("{}", wrapped);
    }

    Ok(())
}
