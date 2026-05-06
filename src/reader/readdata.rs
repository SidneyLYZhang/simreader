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
            // ---------- Lazy CSV（支持自定义分隔符） ----------
            let sep = csv_separator.unwrap_or(b',');
            let lf = LazyCsvReader::new(path)
                .with_separator(sep)
                .has_header(true)
                .finish()?;
            Ok(ReaderEngine::PolarsLazy(lf))
        }

        FileFormat::Json => {
            // ---------- Lazy NDJSON ----------
            // scan_ndjson 直接返回 LazyFrame
            let lf = LazyFrame::scan_ndjson(path, ScanArgsNdJson::default())?;
            Ok(ReaderEngine::PolarsLazy(lf))
        }

        FileFormat::Ipc => {
            // ---------- Lazy Arrow IPC ----------
            let lf = LazyFrame::scan_ipc(path, ScanArgsIpc::default())?;
            Ok(ReaderEngine::PolarsLazy(lf))
        }

        FileFormat::Parquet => {
            // ---------- Lazy Parquet ----------
            let lf = LazyFrame::scan_parquet(path, ScanArgsParquet::default())?;
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
        open_workbook(path).map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;

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
    let merged = workbook
        .merged_regions_by_sheet(&sheet)
        .cloned()
        .unwrap_or_default();

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
    // 情况：工作表为空
    if range.rows().len() == 0 {
        return DataFrame::empty().into();
    }

    // ---------- 提取表头 ----------
    let header_row = range.rows().next().unwrap();
    let mut col_names: Vec<String> = Vec::new();
    // 统计实际有效列数
    let max_cols = range
        .rows()
        .map(|row| row.len())
        .max()
        .unwrap_or(0);

    for col_idx in 0..max_cols {
        let cell_value = header_row.get(col_idx).unwrap_or(&Data::Empty);
        let name = cell_value_to_string(cell_value);
        // 空表头自动补名
        let name = if name.is_empty() {
            format!("column_{}", col_idx + 1)
        } else {
            name
        };
        col_names.push(name);
    }

    // ---------- 构建列向量 ----------
    let mut columns: Vec<Series> = Vec::new();
    for col_idx in 0..max_cols {
        let name = &col_names[col_idx];
        let mut col_data: Vec<String> = Vec::new();

        // 遍历数据行（跳过表头）
        for row in range.rows().skip(1) {
            let cell = row.get(col_idx).unwrap_or(&Data::Empty);
            col_data.push(cell_value_to_string(cell));
        }

        columns.push(Series::new(name, &col_data));
    }

    DataFrame::new(columns)
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
        Data::DurationIso(_) | Data::Duration(_) | Data::Error(_) => String::new(),
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
    mut range: Range<Data>,
    merged_regions: &[(String, String, calamine::Dimensions)],
) -> Range<Data> {
    for (_sheet_name, _path, dim) in merged_regions {
        let start_row = dim.start.0 as usize;
        let start_col = dim.start.1 as usize;
        let end_row = dim.end.0 as usize;
        let end_col = dim.end.1 as usize;

        // 安全获取左上角值
        let top_left_value = {
            let row_idx = start_row.saturating_sub(1);
            let col_idx = start_col.saturating_sub(1);
            if let Some(row) = range.row(row_idx) {
                row.get(col_idx).cloned().unwrap_or(Data::Empty)
            } else {
                Data::Empty
            }
        };

        // 将左上角值复制到合并区域内的所有单元格
        for r in start_row..=end_row {
            let row_idx = r.saturating_sub(1);
            for c in start_col..=end_col {
                let col_idx = c.saturating_sub(1);
                if let Some(row) = range.row_mut(row_idx) {
                    if let Some(cell) = row.get_mut(col_idx) {
                        if *cell == Data::Empty {
                            *cell = top_left_value.clone();
                        }
                    }
                }
            }
        }
    }
    range
}