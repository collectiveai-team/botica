use std::sync::OnceLock;

pub struct Document<'a> {
    pub path: &'a str,
    pub text: &'a str,
    line_starts: OnceLock<Vec<usize>>,
}

impl<'a> Document<'a> {
    pub fn new(path: &'a str, text: &'a str) -> Self {
        Self {
            path,
            text,
            line_starts: OnceLock::new(),
        }
    }

    fn line_starts(&self) -> &[usize] {
        self.line_starts.get_or_init(|| {
            let mut starts = vec![0];
            starts.extend(
                self.text
                    .bytes()
                    .enumerate()
                    .filter(|&(_, byte)| byte == b'\n')
                    .map(|(index, _)| index + 1),
            );
            starts
        })
    }

    /// Number of lines, counted like Python's `sum(1 for _ in open(f))`.
    pub fn line_count(&self) -> usize {
        let newlines = self.text.bytes().filter(|&byte| byte == b'\n').count();
        let unterminated = !self.text.is_empty() && !self.text.ends_with('\n');
        newlines + usize::from(unterminated)
    }

    /// 1-based line and column (in characters) of a byte offset.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let starts = self.line_starts();
        let line = starts.partition_point(|&start| start <= offset);
        let line_start = starts[line - 1];
        let column = self.text[line_start..offset.min(self.text.len())]
            .chars()
            .count();
        (line, column + 1)
    }
}
