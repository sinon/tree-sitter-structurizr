//! Context-aware completion for the bounded LSP slice.

use structurizr_analysis::DocumentSnapshot;
use tower_lsp_server::ls_types::{CompletionItem, CompletionItemKind, Position};

use crate::{convert::positions::position_to_byte_offset, documents::DocumentState};

// =============================================================================
// Fixed vocabulary outside style blocks
// =============================================================================

const FIXED_COMPLETION_ITEMS: &[(&str, &str)] = &[
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

const SHARED_STYLE_PROPERTY_ITEMS: &[(&str, &str)] = &[
    ("color", "Style property"),
    ("colour", "Style property"),
    ("stroke", "Style property"),
    ("opacity", "Style property"),
    ("metadata", "Style property"),
    ("description", "Style property"),
    ("fontSize", "Style property"),
];

const ELEMENT_STYLE_PROPERTY_ITEMS: &[(&str, &str)] = &[
    ("background", "Element style property"),
    ("shape", "Element style property"),
    ("border", "Element style property"),
    ("iconPosition", "Element style property"),
];

const RELATIONSHIP_STYLE_PROPERTY_ITEMS: &[(&str, &str)] = &[
    ("style", "Relationship style property"),
    ("routing", "Relationship style property"),
    ("dashed", "Relationship style property"),
    ("jump", "Relationship style property"),
];

// =============================================================================
// Style-block completion
// =============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionContext {
    FixedVocabulary,
    StyleProperties(StyleBlockKind),
    Suppress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StyleBlockKind {
    Element,
    Relationship,
}

/// Returns completion items that match the current token prefix.
#[must_use]
pub fn completion_items(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    position: Position,
) -> Vec<CompletionItem> {
    let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
        return Vec::new();
    };
    let prefix_start = completion_prefix_start(document.text(), offset);
    let prefix = &document.text()[prefix_start..offset.min(document.text().len())];

    let candidates = match completion_context(document.text(), snapshot, offset, prefix_start) {
        CompletionContext::FixedVocabulary => FIXED_COMPLETION_ITEMS.to_vec(),
        CompletionContext::StyleProperties(kind) => style_property_items(kind),
        CompletionContext::Suppress => Vec::new(),
    };

    candidates
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

fn style_property_items(kind: StyleBlockKind) -> Vec<(&'static str, &'static str)> {
    let mut items = SHARED_STYLE_PROPERTY_ITEMS.to_vec();

    match kind {
        StyleBlockKind::Element => items.extend_from_slice(ELEMENT_STYLE_PROPERTY_ITEMS),
        StyleBlockKind::Relationship => items.extend_from_slice(RELATIONSHIP_STYLE_PROPERTY_ITEMS),
    }

    items
}

fn completion_context(
    text: &str,
    snapshot: &DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> CompletionContext {
    // Style bodies are the one place where the global keyword vocabulary becomes
    // distracting. If we can confidently detect a style-property position, switch
    // to the style tables; otherwise keep the original fixed vocabulary.
    style_block_context(text, snapshot, offset, prefix_start)
        .unwrap_or(CompletionContext::FixedVocabulary)
}

fn style_block_context(
    text: &str,
    snapshot: &DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> Option<CompletionContext> {
    let node = syntax_node_at_offset(text, snapshot, offset, prefix_start)?;
    let mut current = node;
    let mut style_setting = None;
    let mut style_rule_block = None;
    let style_kind = loop {
        match current.kind() {
            "style_setting" if style_setting.is_none() => style_setting = Some(current),
            "style_rule_block" if style_rule_block.is_none() => style_rule_block = Some(current),
            "element_style" => break Some(StyleBlockKind::Element),
            "relationship_style" => break Some(StyleBlockKind::Relationship),
            _ => {}
        }

        current = current.parent()?;
    }?;

    if let Some(style_setting) = style_setting {
        let name = style_setting.child_by_field_name("name")?;
        let cursor_probe = offset.min(text.len());
        let prefix_probe = prefix_start.saturating_sub(1);

        // Once the cursor has moved off the property name and into the value,
        // suppress the global keyword list as well. That keeps `metadata true`
        // or `routing Orthogonal` from showing unrelated workspace keywords.
        return Some(
            if span_contains(name.start_byte(), name.end_byte(), cursor_probe)
                || span_contains(name.start_byte(), name.end_byte(), prefix_probe)
                || cursor_probe == name.end_byte()
            {
                CompletionContext::StyleProperties(style_kind)
            } else {
                CompletionContext::Suppress
            },
        );
    }

    let style_rule_block = style_rule_block?;
    if !span_contains(
        style_rule_block.start_byte(),
        style_rule_block.end_byte(),
        offset,
    ) {
        return None;
    }

    Some(if is_style_property_insertion_point(text, prefix_start) {
        CompletionContext::StyleProperties(style_kind)
    } else {
        CompletionContext::Suppress
    })
}

fn syntax_node_at_offset<'a>(
    text: &str,
    snapshot: &'a DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> Option<tree_sitter::Node<'a>> {
    if text.is_empty() {
        return None;
    }

    let root = snapshot.tree().root_node();
    let last_byte = text.len().saturating_sub(1);
    // Completion is often requested on whitespace or just past the end of a
    // partially typed token. Probe a few nearby offsets so Tree-sitter still
    // gives us a useful enclosing node for context detection.
    let probes = [
        offset.min(last_byte),
        prefix_start.min(last_byte),
        offset.saturating_sub(1).min(last_byte),
        prefix_start.saturating_sub(1).min(last_byte),
    ];

    probes
        .into_iter()
        .find_map(|probe| root.descendant_for_byte_range(probe, probe))
}

fn is_style_property_insertion_point(text: &str, prefix_start: usize) -> bool {
    let safe_start = prefix_start.min(text.len());
    let line_start = text[..safe_start].rfind('\n').map_or(0, |index| index + 1);
    let brace_start = text[..safe_start].rfind('{').map_or(0, |index| index + 1);
    let segment_start = line_start.max(brace_start);

    text[segment_start..safe_start].trim().is_empty()
}

const fn span_contains(start_byte: usize, end_byte: usize, offset: usize) -> bool {
    if start_byte == end_byte {
        offset == start_byte
    } else {
        start_byte <= offset && offset < end_byte
    }
}

fn completion_prefix_start(text: &str, offset: usize) -> usize {
    let safe_offset = offset.min(text.len());
    let bytes = text.as_bytes();

    bytes[..safe_offset]
        .iter()
        .rposition(|byte| !matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'!'))
        .map_or(0, |index| index + 1)
}
