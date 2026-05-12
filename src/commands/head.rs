use std::path::Path;

use crate::commands::util;
use crate::reader::readdata::FileFormat;

pub fn head_data_file(file_path: &str, n: usize, no_name: bool, _line_width: usize, force_csv: bool, csv_separator: Option<u8>, col_selection: Option<&str>) -> anyhow::Result<()> {
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
        let display: Vec<String> = cells.iter().enumerate().map(|(i, c)| {
            let char_count = c.chars().count();
            if char_count > col_widths[i] {
                let truncated: String = c.chars().take(col_widths[i].saturating_sub(3)).collect();
                format!("{}...", truncated)
            } else {
                c.clone()
            }
        }).collect();
        println!("{}", display.join("\t"));
    }

    Ok(())
}

pub fn head_text_file(file_path: &str, n: usize, line_width: usize) -> anyhow::Result<()> {
    let path = Path::new(file_path);
    let mut reader = crate::reader::readtext::FileReader::new(path)?;
    let total = reader.total_lines();
    let count = n.min(total);
    let lines = reader.read_segment(0, count)?;

    for line in &lines {
        let wrapped = util::wrap_line_en(line, line_width);
        println!("{}", wrapped);
    }

    Ok(())
}
