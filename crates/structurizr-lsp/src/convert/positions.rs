//! Position conversion stays in the LSP crate because UTF-16 is a protocol concern.

use line_index::{LineIndex, TextSize, WideEncoding, WideLineCol};
use structurizr_analysis::TextSpan;
use tower_lsp_server::ls_types::{Position, Range};

fn byte_offset_to_position(line_index: &LineIndex, byte_offset: usize) -> Option<Position> {
    let offset = TextSize::from(u32::try_from(byte_offset).ok()?);
    let utf8 = line_index.try_line_col(offset)?;
    let wide = line_index.to_wide(WideEncoding::Utf16, utf8)?;

    Some(Position::new(wide.line, wide.col))
}

/// Converts one byte-offset range into an LSP UTF-16 range for one document.
#[must_use]
pub fn byte_offsets_to_range(
    line_index: &LineIndex,
    start_byte: usize,
    end_byte: usize,
) -> Option<Range> {
    Some(Range::new(
        byte_offset_to_position(line_index, start_byte)?,
        byte_offset_to_position(line_index, end_byte)?,
    ))
}

/// Converts an LSP UTF-16 position into a UTF-8 byte offset for one document.
#[must_use]
pub fn position_to_byte_offset(line_index: &LineIndex, position: Position) -> Option<usize> {
    let utf8 = line_index.to_utf8(
        WideEncoding::Utf16,
        WideLineCol {
            line: position.line,
            col: position.character,
        },
    )?;
    let offset = line_index.offset(utf8)?;

    usize::try_from(u32::from(offset)).ok()
}

/// Converts an analysis span into an LSP range using the document line index.
#[must_use]
pub fn span_to_range(line_index: &LineIndex, span: TextSpan) -> Option<Range> {
    byte_offsets_to_range(line_index, span.start_byte, span.end_byte)
}
