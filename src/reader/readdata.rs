use std::path::Path;

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use polars::prelude::*;

// ============================================================================
// API：对外暴露的统一入口 & 文件格式枚举
// ============================================================================

/// 支持的输入文件格式
#[derive(Debug, Clone, Copy)]
pub enum FileFormat {
    Csv,
    Json,
    Ipc,
    Parquet,
    Excel,
}

/// 一次调用完成对多种文件格式的读取，返回统一的 `LazyFrame`
///
/// # Arguments
/// * `file_path`  - 文件的完整路径
/// * `format`     - 文件格式枚举（Csv/Json/Ipc/Parquet/Excel）
/// * `csv_separator` - CSV 列分隔符（仅 `Csv` 时使用，默认 `b','`）
/// * `sheet_name` - Excel 工作表名（仅 `Excel` 时使用，默认第一个工作表）
///
/// # Errors
/// 当文件不存在、格式不匹配、或读取/解析失败时返回错误
pub fn read_to_lazyframe(
    file_path: &str,
    format: FileFormat,
    csv_separator: Option<u8>,
    sheet_name: Option<String>,
) -> PolarsResult<LazyFrame> {
    // 1. 根据文件格式选择引擎，读取为 DfOrEngine
    let engine = read_with_engine(file_path, format, csv_separator, sheet_name)?;

    // 2. 统一转换为 LazyFrame 返回
    engine_to_lazyframe(engine)
}

// ============================================================================
// 内部：引擎区分 & 读取逻辑
// ============================================================================

/// 内部引擎标识：Polars 原生 scan_* 返回 `LazyFrame`，Calamine 返回 `DataFrame`
enum ReaderEngine {
    PolarsLazy(LazyFrame),
    Calamine(DataFrame),
}

/// 将引擎结果统一转换为 `LazyFrame`
fn engine_to_lazyframe(engine: ReaderEngine) -> PolarsResult<LazyFrame> {
    match engine {
        ReaderEngine::PolarsLazy(lf) => {
            // Polars 原生 lazy frame：直接返回
            Ok(lf)
        }
        ReaderEngine::Calamine(df) => {
            // Calamine 读取到的 eager DataFrame：转为 LazyFrame
            Ok(df.lazy())
        }
    }
}

/// 根据文件格式读取数据，返回引擎枚举
fn read_with_engine(
    file_path: &str,
    format: FileFormat,
    csv_separator: Option<u8>,
    sheet_name: Option<String>,
) -> PolarsResult<ReaderEngine> {
    let path = Path::new(file_path);
    match format {
        FileFormat::Csv => {
            let sep = csv_separator.unwrap_or(b',');
            let lf = LazyCsvReader::new(PlRefPath::from(file_path))
                .with_separator(sep)
                .with_has_header(true)
                .finish()?;
            Ok(ReaderEngine::PolarsLazy(lf))
        }

        FileFormat::Json => {
            let lf = LazyJsonLineReader::new(PlRefPath::from(file_path)).finish()?;
            Ok(ReaderEngine::PolarsLazy(lf))
        }

        FileFormat::Ipc => {
            let lf = LazyFrame::scan_ipc(
                PlRefPath::from(file_path),
                Default::default(),
                Default::default(),
            )?;
            Ok(ReaderEngine::PolarsLazy(lf))
        }

        FileFormat::Parquet => {
            let lf = LazyFrame::scan_parquet(PlRefPath::from(file_path), ScanArgsParquet::default())?;
            Ok(ReaderEngine::PolarsLazy(lf))
        }

        FileFormat::Excel => {
            // ---------- Excel via Calamine + 转为 DataFrame ----------
            let df = read_excel_to_df(path, sheet_name)?;
            Ok(ReaderEngine::Calamine(df))
        }
    }
}

// ============================================================================
// Excel 专项处理（calamine）
// ============================================================================

/// 使用 calamine 读取 Excel 文件，返回 Polars `DataFrame`
///
/// 1. 用 calamine 的 `open_workbook` 打开 xlsx/xls/ods 等格式
/// 2. 根据 `sheet_name` 选择工作表（未指定则取默认工作表名）
/// 3. 调用 `load_merged_regions` 获取合并单元格信息
/// 4. 调用 `worksheet_range` 读取整个工作表数据
/// 5. 调用 `fill_merged_cells` 将合并区域的值向下/向右填充
/// 6. 将 calamine 的 `Range<Data>` 转换为 Polars 的 `DataFrame`
fn read_excel_to_df(
    path: &Path,
    sheet_name: Option<String>,
) -> PolarsResult<DataFrame> {
    // 1. 打开工作簿
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e: calamine::XlsxError| PolarsError::ComputeError(e.to_string().into()))?;

    // 2. 确定要读取的工作表名
    let sheet = sheet_name.unwrap_or_else(|| {
        workbook
            .sheet_names()
            .first()
            .cloned()
            .unwrap_or_else(|| "Sheet1".to_string())
    });

    // 3. 载入合并单元格信息
    workbook
        .load_merged_regions()
        .map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;
    let merged: Vec<(String, String, calamine::Dimensions)> = workbook
        .merged_regions_by_sheet(&sheet)
        .into_iter()
        .map(|(a, b, c)| (a.clone(), b.clone(), c.clone()))
        .collect();

    // 4. 读取整个工作表的单元格范围
    let range: Range<Data> = workbook
        .worksheet_range(&sheet)
        .map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;

    // 5. 填充合并单元格
    let filled_range = fill_merged_cells(range, &merged);

    // 6. 转换为 Polars DataFrame
    calamine_range_to_polars_df(&filled_range)
}

/// 将 calamine 的 `Range<Data>` 转换为 Polars 的 `DataFrame`
fn calamine_range_to_polars_df(range: &Range<Data>) -> PolarsResult<DataFrame> {
    let all_rows: Vec<&[Data]> = range.rows().collect::<Vec<_>>();
    if all_rows.is_empty() {
        return Ok(DataFrame::empty());
    }

    let header_row = all_rows[0];
    let max_cols = all_rows.iter().map(|row| row.len()).max().unwrap_or(0);

    let mut col_names: Vec<String> = Vec::new();
    for col_idx in 0..max_cols {
        let cell_value = header_row.get(col_idx).unwrap_or(&Data::Empty);
        let name = cell_value_to_string(cell_value);
        let name = if name.is_empty() {
            format!("column_{}", col_idx + 1)
        } else {
            name
        };
        col_names.push(name);
    }

    let data_rows = all_rows.len().saturating_sub(1);
    let mut columns: Vec<polars::prelude::Column> = Vec::new();
    for col_idx in 0..max_cols {
        let name = PlSmallStr::from_str(&col_names[col_idx]);

        let col_cells: Vec<&Data> = all_rows.iter().skip(1)
            .map(|row| row.get(col_idx).unwrap_or(&Data::Empty))
            .collect();

        let series = build_typed_series(&name, &col_cells);
        columns.push(polars::prelude::Column::from(series));
    }

    DataFrame::new(data_rows, columns)
}

fn build_typed_series(name: &PlSmallStr, cells: &[&Data]) -> Series {
    let non_empty: Vec<&&Data> = cells.iter().filter(|d| !matches!(d, Data::Empty)).collect();

    if non_empty.is_empty() {
        let empty_strs: Vec<&str> = vec![""; cells.len()];
        return Series::new(name.clone(), &empty_strs);
    }

    let all_bool = non_empty.iter().all(|d| matches!(d, Data::Bool(_)));
    let all_int = non_empty.iter().all(|d| matches!(d, Data::Int(_)));
    let all_float = non_empty.iter().all(|d| matches!(d, Data::Float(_)));
    let all_numeric = non_empty.iter().all(|d| matches!(d, Data::Int(_) | Data::Float(_)));
    let all_datetime = non_empty.iter().all(|d| matches!(d, Data::DateTime(_)));

    if all_bool {
        let values: Vec<Option<bool>> = cells.iter().map(|d| {
            match d {
                Data::Bool(b) => Some(*b),
                _ => None,
            }
        }).collect();
        return Series::new(name.clone(), &values);
    }

    if all_int {
        let values: Vec<Option<i64>> = cells.iter().map(|d| {
            match d {
                Data::Int(i) => Some(*i),
                _ => None,
            }
        }).collect();
        return Series::new(name.clone(), &values);
    }

    if all_float || all_numeric {
        let values: Vec<Option<f64>> = cells.iter().map(|d| {
            match d {
                Data::Float(f) => Some(*f),
                Data::Int(i) => Some(*i as f64),
                _ => None,
            }
        }).collect();
        return Series::new(name.clone(), &values);
    }

    if all_datetime {
        let values: Vec<String> = cells.iter().map(|d| {
            match d {
                Data::DateTime(dt) => format!("{}", dt),
                _ => String::new(),
            }
        }).collect();
        return Series::new(name.clone(), &values);
    }

    let all_string = non_empty.iter().all(|d| matches!(d, Data::String(_)));
    if all_string {
        let all_percent = non_empty.iter().all(|d| {
            if let Data::String(s) = d {
                s.ends_with('%')
            } else {
                false
            }
        });
        if all_percent {
            let values: Vec<Option<f64>> = cells.iter().map(|d| {
                match d {
                    Data::String(s) if s.ends_with('%') => {
                        let num_str = &s[..s.len() - 1];
                        num_str.trim().parse::<f64>().ok().map(|n| n / 100.0)
                    }
                    _ => None,
                }
            }).collect();
            return Series::new(name.clone(), &values);
        }
    }

    let values: Vec<String> = cells.iter().map(|d| cell_value_to_string(d)).collect();
    Series::new(name.clone(), &values)
}

/// 将 calamine 的 `Data` 变量转换为 Rust `String`
fn cell_value_to_string(data: &Data) -> String {
    match data {
        Data::Empty => String::new(),
        Data::Bool(b) => b.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => f.to_string(),
        Data::String(s) => s.clone(),
        Data::DateTime(dt) => format!("{}", dt),
        Data::Error(_) => String::new(),
        _ => String::new(),
    }
}

// ============================================================================
// 合并单元格填充逻辑
// ============================================================================

/// 将 Excel 合并区域的值向下（右）填充至所有被覆盖的单元格
///
/// calamine 默认只在合并区域的左上角保留值，其余位置为 `Empty`。
/// 本函数以左上角值为准，将该值复制到区域内的每一个单元格，
/// 使其成为规整的矩形表格。
fn fill_merged_cells(
    range: Range<Data>,
    merged_regions: &[(String, String, calamine::Dimensions)],
) -> Range<Data> {
    let mut data: Vec<Vec<Data>> = range
        .rows()
        .map(|row| row.to_vec())
        .collect();

    for (_sheet_name, _path, dim) in merged_regions {
        let start_row = dim.start.0 as usize;
        let start_col = dim.start.1 as usize;
        let end_row = dim.end.0 as usize;
        let end_col = dim.end.1 as usize;

        let top_left_value = {
            let row_idx = start_row.saturating_sub(1);
            let col_idx = start_col.saturating_sub(1);
            data.get(row_idx)
                .and_then(|row| row.get(col_idx))
                .cloned()
                .unwrap_or(Data::Empty)
        };

        for r in start_row..=end_row {
            let row_idx = r.saturating_sub(1);
            for c in start_col..=end_col {
                let col_idx = c.saturating_sub(1);
                if let Some(row) = data.get_mut(row_idx) {
                    if let Some(cell) = row.get_mut(col_idx) {
                        if *cell == Data::Empty {
                            *cell = top_left_value.clone();
                        }
                    }
                }
            }
        }
    }

    let cells: Vec<calamine::Cell<Data>> = data
        .into_iter()
        .enumerate()
        .flat_map(|(i, row)| {
            row.into_iter().enumerate().map(move |(j, d)| {
                calamine::Cell::new((i as u32, j as u32), d)
            })
        })
        .collect();
    Range::from_sparse(cells)
}