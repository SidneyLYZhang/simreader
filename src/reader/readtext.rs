use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

/// 高性能文本文件行读取器
///
/// * 自动建立行索引，支持 O(1) 跳转到任意行
/// * 按需分段加载，避免一次性读入大文件
/// * 实现 Iterator，可直接用于 for 循环按行遍历
pub struct FileReader {
    // 带缓冲的读取器，内部可 seek
    reader: BufReader<File>,
    // 行起始字节偏移量索引：index[i] = 第 i 行的第一个字节的绝对偏移
    index: Vec<u64>,
    // 下一次读取将返回的行号（0 起始）
    current_line: usize,
}

impl FileReader {
    /// 打开文本文件并建立行索引
    ///
    /// 索引过程使用缓冲读取并记录每行的精确字节偏移，
    /// 索引完成后自动将读取位置重置到文件开头。
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut index = Vec::new();
        let mut line_buf = String::new();

        loop {
            // 在读取下一行之前记录当前文件流位置（即该行的起始偏移）
            let pos = reader.stream_position()?;
            let bytes_read = reader.read_line(&mut line_buf)?;
            if bytes_read == 0 {
                break; // EOF
            }
            index.push(pos);
            line_buf.clear();
        }

        // 将读取位置重置到文件开头，准备后续读取
        reader.seek(SeekFrom::Start(0))?;

        Ok(FileReader {
            reader,
            index,
            current_line: 0,
        })
    }

    /// 返回文件的总行数
    pub fn total_lines(&self) -> usize {
        self.index.len()
    }

    /// 返回当前即将读取的行号（0 起始）
    pub fn current_line_number(&self) -> usize {
        self.current_line
    }

    /// 快速跳转到第 `line` 行（0 起始）
    ///
    /// 若 `line` 超过总行数，则 seek 到文件末尾，后续读取将返回 EOF。
    pub fn seek_to_line(&mut self, line: usize) -> io::Result<()> {
        if line >= self.index.len() {
            // 超出范围：定位到文件末尾
            self.reader.seek(SeekFrom::End(0))?;
            self.current_line = self.index.len();
        } else {
            self.reader.seek(SeekFrom::Start(self.index[line]))?;
            self.current_line = line;
        }
        Ok(())
    }

    /// 分段加载：从 `start` 行开始读取 `count` 行
    ///
    /// 返回的字符串已去除行尾换行符（`\n` 和 `\r\n`）。
    /// 若实际可读行数不足 `count`，则返回剩余的所有行。
    /// 读取完成后，`current_line` 将指向最后读取行的下一行。
    pub fn read_segment(&mut self, start: usize, count: usize) -> io::Result<Vec<String>> {
        self.seek_to_line(start)?;
        let mut lines = Vec::with_capacity(count);
        for _ in 0..count {
            let mut buf = String::new();
            let len = self.reader.read_line(&mut buf)?;
            if len == 0 {
                break;
            }
            // 去除行尾换行符
            if buf.ends_with('\n') {
                buf.pop();
                if buf.ends_with('\r') {
                    buf.pop();
                }
            }
            lines.push(buf);
            self.current_line += 1;
        }
        Ok(lines)
    }
}

/// 为 &mut FileReader 实现 Iterator，使其可直接用于 for 循环
///
/// 每次迭代返回下一行的内容（已去除换行符），
/// 并自动推进内部行号。
impl<'a> Iterator for &'a mut FileReader {
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf) {
            Ok(0) => None,                     // EOF
            Ok(_) => {
                if buf.ends_with('\n') {
                    buf.pop();
                    if buf.ends_with('\r') {
                        buf.pop();
                    }
                }
                self.current_line += 1;
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}