//! Fixed-vocabulary completion for the initial LSP slice.

use tower_lsp_server::ls_types::{CompletionItem, CompletionItemKind, Position};

use crate::{
    convert::positions::position_to_byte_offset,
    documents::DocumentState,
};

const COMPLETION_ITEMS: &[(&str, &str)] = &[
    ("workspace", "Workspace root"),
    ("model", "Workspace section"),
    ("views", "Workspace section"),
    ("configuration", "Workspace section"),
    ("person", "Core declaration"),
    ("softwareSystem", "Core declaration"),
    ("container", "Core declaration"),
    ("component", "Core declaration"),
    ("!include", "Directive"),
    ("!identifiers", "Directive"),
    ("!docs", "Directive"),
    ("!adrs", "Directive"),
    ("include", "View statement"),
    ("exclude", "View statement"),
    ("autoLayout", "View statement"),
    ("title", "View statement"),
    ("description", "View statement"),
];

#[must_use]
pub fn completion_items(document: &DocumentState, position: Position) -> Vec<CompletionItem> {
    let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
        return Vec::new();
    };

    let prefix = completion_prefix(document.text(), offset);

    COMPLETION_ITEMS
        .iter()
        .filter(|(label, _)| prefix.is_empty() || label.starts_with(prefix))
        .map(|(label, detail)| CompletionItem {
            label: (*label).to_owned(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some((*detail).to_owned()),
            ..CompletionItem::default()
        })
        .collect()
}

fn completion_prefix(text: &str, offset: usize) -> &str {
    let safe_offset = offset.min(text.len());
    let bytes = text.as_bytes();
    let start = bytes[..safe_offset]
        .iter()
        .rposition(|byte| !matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'!'))
        .map_or(0, |index| index + 1);

    &text[start..safe_offset]
}
