//! Transport-agnostic text positions and spans derived from Tree-sitter ranges.

use tree_sitter::{Node, Point, Range};

/// Zero-based text coordinates derived from Tree-sitter points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPoint {
    /// Zero-based line index.
    pub row: usize,
    /// Zero-based UTF-8 column index within the line.
    pub column: usize,
}

impl From<Point> for TextPoint {
    fn from(point: Point) -> Self {
        Self {
            row: point.row,
            column: point.column,
        }
    }
}

/// Byte and point range derived from a Tree-sitter node or range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    /// Inclusive start byte offset.
    pub start_byte: usize,
    /// Exclusive end byte offset.
    pub end_byte: usize,
    /// Zero-based start position.
    pub start_point: TextPoint,
    /// Zero-based end position.
    pub end_point: TextPoint,
}

impl TextSpan {
    #[must_use]
    /// Creates a span that covers the full range of a Tree-sitter node.
    pub fn from_node(node: Node<'_>) -> Self {
        node.range().into()
    }

    #[must_use]
    /// Returns whether the span covers zero bytes.
    pub const fn is_empty(&self) -> bool {
        self.start_byte == self.end_byte
    }
}

impl From<Range> for TextSpan {
    fn from(range: Range) -> Self {
        Self {
            start_byte: range.start_byte,
            end_byte: range.end_byte,
            start_point: range.start_point.into(),
            end_point: range.end_point.into(),
        }
    }
}
