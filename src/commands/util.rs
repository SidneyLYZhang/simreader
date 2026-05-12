use std::collections::HashMap;

use polars::prelude::*;
use regex::Regex;

pub fn detect_file_format(file_path: &str) -> Option<crate::reader::readdata::FileFormat> {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".csv") {
        Some(crate::reader::readdata::FileFormat::Csv)
    } else if lower.ends_with(".tsv") {
        Some(crate::reader::readdata::FileFormat::Csv)
    } else if lower.ends_with(".json") || lower.ends_with(".ndjson") || lower.ends_with(".jsonl") {
        Some(crate::reader::readdata::FileFormat::Json)
    } else if lower.ends_with(".ipc") || lower.ends_with(".arrow") {
        Some(crate::reader::readdata::FileFormat::Ipc)
    } else if lower.ends_with(".parquet") {
        Some(crate::reader::readdata::FileFormat::Parquet)
    } else if lower.ends_with(".xls") || lower.ends_with(".xlsx") || lower.ends_with(".ods") {
        Some(crate::reader::readdata::FileFormat::Excel)
    } else {
        None
    }
}

pub fn is_data_file(file_path: &str) -> bool {
    detect_file_format(file_path).is_some()
}

pub fn is_text_file(file_path: &str) -> bool {
    !is_data_file(file_path)
}

pub fn csv_separator_for_file(file_path: &str) -> Option<u8> {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".tsv") {
        Some(b'\t')
    } else {
        None
    }
}

lazy_static::lazy_static! {
    static ref HAN_RE: Regex = Regex::new(r"\p{Han}").unwrap();
    static ref EN_WORD_RE: Regex = Regex::new(r"[a-zA-Z0-9]+").unwrap();
    static ref PUNCT_RE: Regex = Regex::new(r"[[:punct:]]").unwrap();
}

#[derive(Debug, Clone)]
pub struct WordCount {
    pub en_words: usize,
    pub cn_chars: usize,
    pub total: usize,
}

impl WordCount {
    pub fn from_text(text: &str) -> Self {
        let cn_chars = HAN_RE.find_iter(text).count();
        let cleaned = PUNCT_RE.replace_all(text, " ");
        let en_words = EN_WORD_RE.find_iter(&cleaned).count();
        Self {
            en_words,
            cn_chars,
            total: en_words + cn_chars,
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }
}

pub fn en_words_only(text: &str) -> usize {
    let cleaned = PUNCT_RE.replace_all(text, " ");
    EN_WORD_RE.find_iter(&cleaned).count()
}

pub fn cn_chars_only(text: &str) -> usize {
    HAN_RE.find_iter(text).count()
}

pub fn clean_punct(text: &str) -> String {
    PUNCT_RE.replace_all(text, "").to_string()
}

pub fn total_words(text: &str) -> usize {
    WordCount::from_text(text).total
}

pub fn total_words_no_punct(text: &str) -> usize {
    let stripped = PUNCT_RE.replace_all(text, "");
    WordCount::from_text(&stripped).total
}

pub fn wrap_text(text: &str, line_width: usize, is_first_para_line: bool) -> String {
    if line_width == 0 {
        return text.to_string();
    }

    let effective_width = if is_first_para_line {
        let trimmed_start = text.len() - text.trim_start().len();
        let chars: Vec<char> = text.chars().collect();
        let mut visible = 0usize;
        for &ch in &chars[trimmed_start..] {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if visible + w > line_width {
                break;
            }
            visible += w;
        }
        let leading_spaces = trimmed_start;
        leading_spaces + visible
    } else {
        line_width
    };

    let leading = text.len() - text.trim_start().len();
    let trimmed = text.trim_start();
    let chars: Vec<char> = trimmed.chars().collect();

    if chars.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut line_start = 0usize;
    let mut current_width = 0usize;

    for i in 0..chars.len() {
        let ch = chars[i];
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);

        if current_width + ch_width > effective_width && current_width > 0 {
            let line: String = chars[line_start..i].iter().collect();
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&" ".repeat(leading));
            result.push_str(&line);
            line_start = i;
            current_width = ch_width;
        } else {
            current_width += ch_width;
        }
    }

    if line_start < chars.len() {
        let line: String = chars[line_start..].iter().collect();
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&" ".repeat(leading));
        result.push_str(&line);
    }

    result
}

pub fn wrap_text_en(text: &str, line_width: usize) -> String {
    if line_width == 0 {
        return text.to_string();
    }

    let leading = text.len() - text.trim_start().len();
    let trimmed = text.trim_start();

    if trimmed.is_empty() {
        return String::new();
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for word in &words {
        let word_width = word.chars().map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)).sum::<usize>();

        if current_line.is_empty() {
            current_line = word.to_string();
        } else if unicode_width::UnicodeWidthStr::width(current_line.as_str()) + 1 + word_width <= line_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    let prefix = " ".repeat(leading);
    lines
        .into_iter()
        .map(|l| format!("{}{}", prefix, l))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn wrap_line_en(text: &str, line_width: usize) -> String {
    wrap_text_en(text, line_width)
}

pub fn count_soft_lines(text: &str, line_width: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let paragraphs = text.split("\n\n");
    let mut total = 0usize;
    for para in paragraphs {
        if para.is_empty() {
            continue;
        }
        for (i, line) in para.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if i > 0 {
                    total += 1;
                }
                continue;
            }
            let wrapped = wrap_line_en(trimmed, line_width);
            total += wrapped.lines().count();
        }
    }
    total
}

pub fn count_paragraphs(text: &str) -> usize {
    text.split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .count()
}

#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub name: String,
    pub dtype: String,
    pub is_numeric: bool,
    pub count: usize,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub mode: Option<String>,
    pub std_dev: Option<f64>,
    pub variance: Option<f64>,
    pub skewness: Option<f64>,
    pub kurtosis: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub zero_count: usize,
    pub none_count: usize,
    pub na_count: usize,
    pub non_zero_valid_count: usize,
    pub word_count: Option<WordCount>,
    pub top_freq_word: Option<String>,
    pub top_freq_content: Option<(String, usize)>,
    pub longest: Option<String>,
    pub shortest: Option<String>,
}

pub fn compute_column_stats(df: &DataFrame, col_index: usize, col_name: &str, full: bool) -> ColumnStats {
    let col = &df.columns()[col_index];
    let polars_dtype = col.dtype();
    let dtype = format!("{:?}", polars_dtype);
    let is_numeric = polars_dtype.is_numeric();
    let count = col.len();

    let none_count = col.null_count();

    let (effective_numeric, na_count, mean, median, std_dev, variance, skewness, kurtosis, min, max, zero_count, non_zero_valid_count) =
        if is_numeric {
            let na = col
                .f64()
                .map(|s| s.into_iter().filter(|v| v.is_none() || (v.is_some_and(|x| x.is_nan()))).count())
                .unwrap_or(0);
            let stats = compute_numeric_stats(col, full);
            (true, na, stats.0, stats.1, stats.2, stats.3, stats.4, stats.5, stats.6, stats.7, stats.8, stats.9)
        } else if let Some(string_stats) = try_numeric_from_string_col(col, full) {
            string_stats
        } else {
            (false, 0, None, None, None, None, None, None, None, None, 0, 0)
        };

    let (word_count, top_freq_word, top_freq_content, longest, shortest) =
        if !effective_numeric && full {
            compute_text_stats(col)
        } else {
            (None, None, None, None, None)
        };

    let mode = if full {
        compute_mode(col)
    } else {
        None
    };

    ColumnStats {
        name: col_name.to_string(),
        dtype: if effective_numeric && !is_numeric {
            format!("{} (推断为数值)", dtype)
        } else {
            dtype
        },
        is_numeric: effective_numeric,
        count,
        mean,
        median,
        mode,
        std_dev,
        variance,
        skewness,
        kurtosis,
        min,
        max,
        zero_count,
        none_count,
        na_count,
        non_zero_valid_count,
        word_count,
        top_freq_word,
        top_freq_content,
        longest,
        shortest,
    }
}

fn try_numeric_from_string_col(
    col: &Column,
    full: bool,
) -> Option<(bool, usize, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, usize, usize)> {
    let strings: Vec<Option<&str>> = col.str()
        .ok()?
        .into_iter()
        .collect();

    let total = strings.len();
    if total == 0 {
        return None;
    }

    let parsed: Vec<Option<f64>> = strings.iter().map(|opt_s| {
        opt_s.and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }
            trimmed.parse::<f64>().ok().filter(|x| x.is_finite())
        })
    }).collect();

    let non_empty_count = strings.iter().filter(|s| s.is_some_and(|v| !v.trim().is_empty())).count();
    if non_empty_count == 0 {
        return None;
    }

    let parse_success_count = parsed.iter().filter(|v| v.is_some()).count();
    let success_rate = parse_success_count as f64 / non_empty_count as f64;

    if success_rate < 0.9 {
        return None;
    }

    let na_count = parsed.iter().filter(|v| v.is_none()).count();

    let valid: Vec<f64> = parsed.iter().filter_map(|v| *v).collect();
    if valid.is_empty() {
        return Some((true, na_count, None, None, None, None, None, None, None, None, 0, 0));
    }

    let n = valid.len() as f64;
    let sum: f64 = valid.iter().sum();
    let mean = sum / n;
    let zero_count = valid.iter().filter(|&&x| x == 0.0).count();
    let non_zero_valid_count = valid.iter().filter(|&&x| x != 0.0).count();

    if !full {
        let min = valid.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = valid.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        return Some((true, na_count, Some(mean), None, None, None, None, None, Some(min), Some(max), zero_count, non_zero_valid_count));
    }

    let mut sorted = valid.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    let variance = valid.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    let m3 = valid.iter().map(|x| (x - mean).powi(3)).sum::<f64>() / n;
    let m4 = valid.iter().map(|x| (x - mean).powi(4)).sum::<f64>() / n;
    let skewness = if std_dev > 0.0 { m3 / std_dev.powi(3) } else { 0.0 };
    let kurtosis = if variance > 0.0 { m4 / variance.powi(2) - 3.0 } else { 0.0 };
    let min = *sorted.first().unwrap();
    let max = *sorted.last().unwrap();

    Some((true, na_count, Some(mean), Some(median), Some(std_dev), Some(variance), Some(skewness), Some(kurtosis), Some(min), Some(max), zero_count, non_zero_valid_count))
}

fn compute_numeric_stats(col: &Column, full: bool) -> (
    Option<f64>, Option<f64>, Option<f64>, Option<f64>,
    Option<f64>, Option<f64>, Option<f64>, Option<f64>,
    usize, usize,
) {
    let values: Vec<Option<f64>> = col
        .f64()
        .map(|s| s.into_iter().collect())
        .unwrap_or_default();

    let valid: Vec<f64> = values.iter().filter_map(|v| {
        match v {
            Some(x) if x.is_finite() => Some(*x),
            _ => None,
        }
    }).collect();

    if valid.is_empty() {
        return (None, None, None, None, None, None, None, None, 0, 0);
    }

    let n = valid.len() as f64;
    let sum: f64 = valid.iter().sum();
    let mean = sum / n;

    let zero_count = valid.iter().filter(|&&x| x == 0.0).count();
    let non_zero_valid_count = valid.iter().filter(|&&x| x != 0.0).count();

    if !full {
        let min = valid.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = valid.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        return (Some(mean), None, None, None, None, None, Some(min), Some(max), zero_count, non_zero_valid_count);
    }

    let mut sorted = valid.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    let variance = valid.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();

    let m3 = valid.iter().map(|x| (x - mean).powi(3)).sum::<f64>() / n;
    let m4 = valid.iter().map(|x| (x - mean).powi(4)).sum::<f64>() / n;

    let skewness = if std_dev > 0.0 { m3 / std_dev.powi(3) } else { 0.0 };
    let kurtosis = if variance > 0.0 { m4 / variance.powi(2) - 3.0 } else { 0.0 };

    let min = *sorted.first().unwrap();
    let max = *sorted.last().unwrap();

    (
        Some(mean), Some(median), Some(std_dev), Some(variance),
        Some(skewness), Some(kurtosis), Some(min), Some(max),
        zero_count, non_zero_valid_count,
    )
}

fn compute_text_stats(col: &Column) -> (
    Option<WordCount>,
    Option<String>,
    Option<(String, usize)>,
    Option<String>,
    Option<String>,
) {
    let values: Vec<String> = col
        .str()
        .map(|s| s.into_iter().map(|v| v.unwrap_or("").to_string()).collect())
        .unwrap_or_default();

    let non_empty: Vec<&str> = values.iter().map(|s| s.as_str()).filter(|s| !s.is_empty()).collect();

    if non_empty.is_empty() {
        return (None, None, None, None, None);
    }

    let all_text = non_empty.join(" ");
    let wc = WordCount::from_text(&all_text);

    let mut freq_map: HashMap<&str, usize> = HashMap::new();
    for text in &non_empty {
        for word in text.split_whitespace() {
            let cleaned: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
            if !cleaned.is_empty() {
                *freq_map.entry(Box::leak(cleaned.into_boxed_str())).or_insert(0) += 1;
            }
        }
    }
    let top_freq_word = freq_map
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(&word, _)| word.to_string());

    let mut content_freq: HashMap<&str, usize> = HashMap::new();
    for text in &non_empty {
        *content_freq.entry(text).or_insert(0) += 1;
    }
    let top_freq_content = content_freq
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(&content, &count)| (content.to_string(), count));

    let longest = non_empty.iter().max_by_key(|s| s.len()).map(|s| s.to_string());
    let shortest = non_empty.iter().min_by_key(|s| s.len()).map(|s| s.to_string());

    (Some(wc), top_freq_word, top_freq_content, longest, shortest)
}

fn compute_mode(col: &Column) -> Option<String> {
    if col.dtype().is_numeric() {
        let values: Vec<Option<f64>> = col.f64().map(|s| s.into_iter().collect()).unwrap_or_default();
        let mut counts: HashMap<String, usize> = HashMap::new();
        for v in values.iter().flatten() {
            let key = format!("{}", v);
            *counts.entry(key).or_insert(0) += 1;
        }
        counts.into_iter().max_by_key(|(_, c)| *c).map(|(k, _)| k)
    } else {
        let values: Vec<String> = col
            .str()
            .map(|s| s.into_iter().map(|v| v.unwrap_or("").to_string()).collect())
            .unwrap_or_default();
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for v in &values {
            if !v.is_empty() {
                *counts.entry(v.as_str()).or_insert(0) += 1;
            }
        }
        counts.into_iter().max_by_key(|(_, c)| *c).map(|(k, _)| k.to_string())
    }
}

pub fn transposed_stats(df: &DataFrame, use_first_col_as_name: bool, _full: bool) -> Vec<ColumnStats> {
    let mut results = Vec::new();
    let n_rows = df.height();

    for row_idx in 0..n_rows {
        let row = df.get_row(row_idx).unwrap();
        let row_name = if use_first_col_as_name {
            row.0[0].to_string()
        } else {
            format!("{}", row_idx)
        };

        let values: Vec<String> = row.0.iter().map(|v| v.to_string()).collect();

        let all_text = values.join(" ");
        let wc = WordCount::from_text(&all_text);

        let is_all_numeric = values.iter().all(|v| v.parse::<f64>().is_ok());
        let (mean, median, mode, std_dev, variance, skewness, kurtosis, min, max, zero_count, _none_count, _na_count, _nz) = if is_all_numeric && values.iter().filter(|v| v.parse::<f64>().is_ok_and(|x| x.is_finite())).count() > 0 {
            let nums: Vec<f64> = values.iter().filter_map(|v| v.parse::<f64>().ok().filter(|x| x.is_finite())).collect();
            let n = nums.len() as f64;
            let sum: f64 = nums.iter().sum();
            let m = sum / n;
            let zc = nums.iter().filter(|&&x| x == 0.0).count();
            let nz = nums.iter().filter(|&&x| x != 0.0).count();

            let mut sorted = nums.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let med = if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            };
            let var = nums.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n;
            let sd = var.sqrt();
            let m3 = nums.iter().map(|x| (x - m).powi(3)).sum::<f64>() / n;
            let m4 = nums.iter().map(|x| (x - m).powi(4)).sum::<f64>() / n;
            let sk = if sd > 0.0 { m3 / sd.powi(3) } else { 0.0 };
            let ku = if var > 0.0 { m4 / var.powi(2) - 3.0 } else { 0.0 };

            (Some(m), Some(med), None, Some(sd), Some(var), Some(sk), Some(ku), Some(*sorted.first().unwrap()), Some(*sorted.last().unwrap()), zc, 0, 0, nz)
        } else {
            (None, None, None, None, None, None, None, None, None, 0, 0, 0, 0)
        };

        let mut content_freq: HashMap<&str, usize> = HashMap::new();
        for v in &values {
            if !v.is_empty() {
                *content_freq.entry(v.as_str()).or_insert(0) += 1;
            }
        }
        let top_freq_content = content_freq
            .iter()
            .max_by_key(|(_, count)| **count)
            .map(|(&content, &count)| (content.to_string(), count));

        results.push(ColumnStats {
            name: row_name,
            dtype: if is_all_numeric { "numeric".into() } else { "str".into() },
            is_numeric: is_all_numeric,
            count: values.len(),
            mean,
            median,
            mode,
            std_dev,
            variance,
            skewness,
            kurtosis,
            min,
            max,
            zero_count,
            none_count: 0,
            na_count: 0,
            non_zero_valid_count: 0,
            word_count: Some(wc),
            top_freq_word: None,
            top_freq_content,
            longest: None,
            shortest: None,
        });
    }

    results
}

#[derive(Debug, Clone)]
pub enum ColSelection {
    Indices(Vec<usize>),
    Names(Vec<String>),
}

pub fn parse_col_selection(input: &str) -> anyhow::Result<ColSelection> {
    let input = input.trim();
    if input.is_empty() {
        anyhow::bail!("--col 参数不能为空");
    }

    if let Some((start, end)) = parse_range(input) {
        if start > end {
            anyhow::bail!("列号范围无效: 起始列 {} 大于结束列 {}", start, end);
        }
        return Ok(ColSelection::Indices((start..=end).collect()));
    }

    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        anyhow::bail!("--col 参数格式无效: '{}'", input);
    }

    let all_numeric = parts.iter().all(|p| p.parse::<usize>().is_ok());

    if all_numeric {
        let indices: Vec<usize> = parts.iter().map(|p| p.parse::<usize>().unwrap()).collect();
        Ok(ColSelection::Indices(indices))
    } else {
        let names: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
        Ok(ColSelection::Names(names))
    }
}

fn parse_range(input: &str) -> Option<(usize, usize)> {
    let colon_pos = input.find(':')?;
    if input.matches(':').count() != 1 {
        return None;
    }
    let start_str = &input[..colon_pos];
    let end_str = &input[colon_pos + 1..];
    let start: usize = start_str.parse().ok()?;
    let end: usize = end_str.parse().ok()?;
    Some((start, end))
}

pub fn resolve_col_selection(df: &polars::prelude::DataFrame, col_sel: &ColSelection) -> anyhow::Result<Vec<String>> {
    match col_sel {
        ColSelection::Indices(indices) => {
            let headers = df.get_column_names();
            let names: Vec<String> = indices.iter().map(|&i| {
                headers.get(i)
                    .ok_or_else(|| anyhow::anyhow!("列索引 {} 超出范围 (共 {} 列)", i, headers.len()))
                    .map(|s| s.to_string())
            }).collect::<Result<_, _>>()?;
            Ok(names)
        }
        ColSelection::Names(names) => {
            let headers: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
            for name in names {
                if !headers.contains(&name.as_str()) {
                    anyhow::bail!("列名 '{}' 不存在", name);
                }
            }
            Ok(names.clone())
        }
    }
}
