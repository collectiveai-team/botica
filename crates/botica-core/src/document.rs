use std::sync::OnceLock;

pub struct Document<'a> {
    pub path: &'a str,
    bytes: &'a [u8],
    text: Option<&'a str>,
    line_starts: OnceLock<Vec<usize>>,
}

impl<'a> Document<'a> {
    pub fn new(path: &'a str, text: &'a str) -> Self {
        Self {
            path,
            bytes: text.as_bytes(),
            text: Some(text),
            line_starts: OnceLock::new(),
        }
    }

    /// A file as it is on disk: `text()` is `None` when it is not UTF-8.
    pub fn from_bytes(path: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            path,
            bytes,
            text: std::str::from_utf8(bytes).ok(),
            line_starts: OnceLock::new(),
        }
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// The contents as text; `None` when the file is not UTF-8.
    pub fn text(&self) -> Option<&'a str> {
        self.text
    }

    fn line_starts(&self) -> &[usize] {
        self.line_starts.get_or_init(|| {
            let mut starts = vec![0];
            starts.extend(
                self.bytes
                    .iter()
                    .enumerate()
                    .filter(|&(_, &byte)| byte == b'\n')
                    .map(|(index, _)| index + 1),
            );
            starts
        })
    }

    /// Number of lines, counted like Python's `sum(1 for _ in open(f))`, on the bytes: the file
    /// need not be UTF-8.
    pub fn line_count(&self) -> usize {
        let newlines = self.bytes.iter().filter(|&&byte| byte == b'\n').count();
        let unterminated = self.bytes.last().is_some_and(|&byte| byte != b'\n');
        newlines + usize::from(unterminated)
    }

    /// 1-based line and column of a byte offset. The column counts characters, or bytes in a
    /// file that is not UTF-8.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let starts = self.line_starts();
        let line = starts.partition_point(|&start| start <= offset);
        let line_start = starts[line - 1];
        let offset = offset.min(self.bytes.len());
        let column = match self.text {
            Some(text) => text[line_start..offset].chars().count(),
            None => offset - line_start,
        };
        (line, column + 1)
    }
}
