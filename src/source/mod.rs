//! Source files, spans and the fixed-format reader.

mod reader;

pub use reader::{ReaderOptions, read_fixed_format};

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub usize);

/// A byte range inside one original source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file: FileId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(file: FileId, start: usize, end: usize) -> Self {
        Span { file, start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Where a byte of logical text came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub file: FileId,
    pub offset: usize,
}

#[derive(Debug)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
    line_starts: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn add_file(&mut self, name: impl Into<String>, text: String) -> FileId {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        self.files.push(SourceFile {
            name: name.into(),
            text,
            line_starts,
        });
        FileId(self.files.len() - 1)
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0]
    }

    /// 1-based line and column (counted in characters) of a byte offset.
    pub fn line_col(&self, id: FileId, offset: usize) -> (usize, usize) {
        let file = self.file(id);
        let line = file.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = file.line_starts[line];
        let col = file.text[start..offset.min(file.text.len())]
            .chars()
            .count()
            + 1;
        (line + 1, col)
    }

    pub fn named_source(&self, id: FileId) -> miette::NamedSource<String> {
        let file = self.file(id);
        miette::NamedSource::new(file.name.clone(), file.text.clone())
    }
}

/// Text with columns, indicators and comments removed, plus a per-byte map back
/// to the original files.
#[derive(Debug, Default, Clone)]
pub struct LogicalText {
    pub text: String,
    pub origins: Vec<Origin>,
}

impl LogicalText {
    /// Appends a character; each of its bytes maps to consecutive original offsets.
    pub fn push_char(&mut self, c: char, file: FileId, offset: usize) {
        self.text.push(c);
        for k in 0..c.len_utf8() {
            self.origins.push(Origin {
                file,
                offset: offset + k,
            });
        }
    }

    /// Appends a character that does not exist in the original file.
    pub fn push_synthetic(&mut self, c: char, origin: Origin) {
        self.text.push(c);
        self.origins
            .extend(std::iter::repeat_n(origin, c.len_utf8()));
    }

    pub fn extend_from(&mut self, other: &LogicalText, range: Range<usize>) {
        self.text.push_str(&other.text[range.clone()]);
        self.origins.extend_from_slice(&other.origins[range]);
    }

    /// Original span covering a non-empty byte range of the logical text.
    pub fn span(&self, start: usize, end: usize) -> Span {
        debug_assert!(start < end && end <= self.text.len());
        let first = self.origins[start];
        let last = self.origins[end - 1];
        Span::new(first.file, first.offset, last.offset + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_is_one_based() {
        let mut map = SourceMap::default();
        let id = map.add_file("a", "ab\ncd\n\nx".to_string());
        assert_eq!(map.line_col(id, 0), (1, 1));
        assert_eq!(map.line_col(id, 1), (1, 2));
        assert_eq!(map.line_col(id, 3), (2, 1));
        assert_eq!(map.line_col(id, 4), (2, 2));
        assert_eq!(map.line_col(id, 6), (3, 1));
        assert_eq!(map.line_col(id, 7), (4, 1));
    }

    #[test]
    fn line_col_counts_characters_not_bytes() {
        let mut map = SourceMap::default();
        let id = map.add_file("a", "ção x".to_string());
        assert_eq!(map.line_col(id, "ção ".len()), (1, 5));
    }

    #[test]
    fn logical_text_maps_multibyte_chars() {
        let mut t = LogicalText::default();
        t.push_char('ç', FileId(0), 10);
        assert_eq!(t.text.len(), 2);
        assert_eq!(t.origins[0].offset, 10);
        assert_eq!(t.origins[1].offset, 11);
        assert_eq!(t.span(0, 2), Span::new(FileId(0), 10, 12));
    }

    #[test]
    fn synthetic_chars_reuse_origin() {
        let mut t = LogicalText::default();
        let o = Origin {
            file: FileId(1),
            offset: 5,
        };
        t.push_synthetic('\n', o);
        assert_eq!(t.origins, vec![o]);
    }

    #[test]
    fn extend_from_copies_range() {
        let mut a = LogicalText::default();
        a.push_char('x', FileId(0), 0);
        a.push_char('y', FileId(0), 1);
        let mut b = LogicalText::default();
        b.extend_from(&a, 1..2);
        assert_eq!(b.text, "y");
        assert_eq!(b.origins[0].offset, 1);
    }
}
