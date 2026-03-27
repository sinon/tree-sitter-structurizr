//! Transport-agnostic text positions and spans derived from Tree-sitter ranges.

use tree_sitter::{Node, Point, Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPoint {
    pub row: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: TextPoint,
    pub end_point: TextPoint,
}

impl TextSpan {
    #[must_use]
    pub fn from_node(node: Node<'_>) -> Self {
        node.range().into()
    }

    #[must_use]
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
