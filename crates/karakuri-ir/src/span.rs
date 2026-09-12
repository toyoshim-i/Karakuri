//! Byte offsets into `.kir` source, kept on every node so that errors can point
//! at the text an LLM wrote rather than at a node type.

/// A half-open byte range `[start, end)` into the source a `Proc` was parsed
/// from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const EMPTY: Span = Span { start: 0, end: 0 };

    pub fn new(start: u32, end: u32) -> Span {
        debug_assert!(start <= end, "span start must not exceed end");
        Span { start, end }
    }

    /// The smallest span covering both. Used to give a compound expression the
    /// extent of its operands.
    pub fn join(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn text(self, src: &str) -> &str {
        &src[self.start as usize..self.end as usize]
    }
}

/// A 1-indexed position, for display only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

/// Resolve a byte offset to a line and column.
///
/// Linear in the length of the source. Only ever called on an error path, so
/// there is nothing to index ahead of time.
pub fn line_col(src: &str, offset: u32) -> LineCol {
    let offset = (offset as usize).min(src.len());
    let mut line = 1;
    let mut line_start = 0;
    for (i, b) in src.as_bytes()[..offset].iter().enumerate() {
        if *b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    LineCol {
        line,
        col: src[line_start..offset].chars().count() as u32 + 1,
    }
}

/// The source line containing `offset`, without its terminator.
pub fn line_text(src: &str, offset: u32) -> &str {
    let offset = (offset as usize).min(src.len());
    let start = src[..offset].rfind('\n').map_or(0, |i| i + 1);
    let end = src[offset..].find('\n').map_or(src.len(), |i| offset + i);
    &src[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_counts_from_one() {
        let src = "abc\ndef\n";
        assert_eq!(line_col(src, 0), LineCol { line: 1, col: 1 });
        assert_eq!(line_col(src, 4), LineCol { line: 2, col: 1 });
        assert_eq!(line_col(src, 6), LineCol { line: 2, col: 3 });
    }

    #[test]
    fn line_text_excludes_terminator() {
        let src = "abc\ndef\nghi";
        assert_eq!(line_text(src, 5), "def");
        assert_eq!(line_text(src, 0), "abc");
        assert_eq!(line_text(src, 9), "ghi");
    }

    #[test]
    fn join_covers_both() {
        assert_eq!(Span::new(2, 4).join(Span::new(8, 9)), Span::new(2, 9));
    }
}
