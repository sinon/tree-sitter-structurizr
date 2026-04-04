//! Context-aware completion for the bounded LSP slice.

use std::collections::BTreeMap;

use structurizr_analysis::{
    DirectiveContainer, DocumentId, DocumentSnapshot, ElementIdentifierMode, IdentifierMode,
    Symbol, SymbolHandle, SymbolKind, WorkspaceFacts, WorkspaceIndex, WorkspaceInstanceId,
};
use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Position, Range, TextEdit,
};

use crate::{
    convert::positions::{byte_offsets_to_range, position_to_byte_offset},
    documents::DocumentState,
};

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

// These property groups and value tables intentionally mirror the finite values
// already accepted by the grammar. The completion logic keeps them in Rust to
// stay syntax-backed, and the unit tests below guard that they stay aligned
// with grammar.js.
const COLOR_STYLE_VALUE_PROPERTIES: &[&str] = &["background", "color", "colour", "stroke"];

const BOOLEAN_STYLE_VALUE_PROPERTIES: &[&str] = &["metadata", "description"];

const BORDER_STYLE_VALUE_PROPERTIES: &[&str] = &["border"];

const SHAPE_STYLE_VALUE_PROPERTIES: &[&str] = &["shape"];

const NAMED_COLOR_VALUES: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

const BOOLEAN_STYLE_VALUE_VALUES: &[&str] = &["true", "false"];

const BORDER_STYLE_VALUE_VALUES: &[&str] = &["Solid", "Dashed", "Dotted"];

const SHAPE_STYLE_VALUE_VALUES: &[&str] = &[
    "Box",
    "RoundedBox",
    "Circle",
    "Ellipse",
    "Hexagon",
    "Diamond",
    "Cylinder",
    "Bucket",
    "Pipe",
    "Person",
    "Robot",
    "Folder",
    "WebBrowser",
    "Window",
    "Terminal",
    "Shell",
    "MobileDevicePortrait",
    "MobileDeviceLandscape",
    "Component",
];

const CORE_ELEMENT_KINDS: &[SymbolKind] = &[
    SymbolKind::Person,
    SymbolKind::SoftwareSystem,
    SymbolKind::Container,
    SymbolKind::Component,
];

// =============================================================================
// Completion context detection
// =============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
enum CompletionContext {
    FixedVocabulary,
    StyleProperties(StyleBlockKind),
    StyleValues(StyleValueCompletionContext),
    RelationshipIdentifier(RelationshipCompletionContext),
    FreshRelationshipSource,
    Suppress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StyleBlockKind {
    Element,
    Relationship,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StyleValueCompletionContext {
    kind: StyleValueKind,
    hash_prefixed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StyleValueKind {
    NamedColor,
    Boolean,
    Border,
    Shape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StyleValuePrefixMode {
    Plain,
    HashPrefixed,
}

impl StyleValueKind {
    const fn candidates(self) -> &'static [&'static str] {
        match self {
            Self::NamedColor => NAMED_COLOR_VALUES,
            Self::Boolean => BOOLEAN_STYLE_VALUE_VALUES,
            Self::Border => BORDER_STYLE_VALUE_VALUES,
            Self::Shape => SHAPE_STYLE_VALUE_VALUES,
        }
    }

    const fn detail(self) -> &'static str {
        match self {
            Self::NamedColor => "Named colour",
            Self::Boolean => "Boolean style value",
            Self::Border => "Border style value",
            Self::Shape => "Shape style value",
        }
    }

    const fn completion_kind(self) -> CompletionItemKind {
        match self {
            Self::NamedColor => CompletionItemKind::COLOR,
            Self::Boolean => CompletionItemKind::VALUE,
            Self::Border | Self::Shape => CompletionItemKind::ENUM_MEMBER,
        }
    }

    const fn supports_quoted_values(self) -> bool {
        matches!(self, Self::Border | Self::Shape)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RelationshipCompletionContext {
    endpoint: RelationshipEndpoint,
    source_text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelationshipEndpoint {
    Source,
    Destination,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IdentifierCompletionCandidate {
    label: String,
    detail: String,
}

enum WorkspaceCompletionOutcome {
    Candidates(BTreeMap<String, IdentifierCompletionCandidate>),
    NoWorkspaceContext,
    Suppress,
}

impl CompletionContext {
    const fn allows_quoted_completion(&self) -> bool {
        matches!(
            self,
            Self::StyleValues(StyleValueCompletionContext { kind, .. })
                if kind.supports_quoted_values()
        )
    }
}

/// Returns completion items that match the current token prefix.
#[must_use]
pub fn completion_items(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
    position: Position,
) -> Vec<CompletionItem> {
    let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
        return Vec::new();
    };
    let prefix_start = completion_prefix_start(document.text(), offset);
    let prefix = &document.text()[prefix_start..offset.min(document.text().len())];
    let edit_range = byte_offsets_to_range(document.line_index(), prefix_start, offset);

    match completion_context(document.text(), snapshot, offset, prefix_start) {
        CompletionContext::FixedVocabulary => {
            keyword_completion_items(FIXED_COMPLETION_ITEMS.to_vec(), edit_range, prefix)
        }
        CompletionContext::StyleProperties(kind) => {
            keyword_completion_items(style_property_items(kind), edit_range, prefix)
        }
        CompletionContext::StyleValues(context) => {
            style_value_completion_items(context, edit_range, prefix)
        }
        CompletionContext::RelationshipIdentifier(context) => relationship_identifier_items(
            document,
            snapshot,
            workspace_facts,
            &context,
            prefix,
            edit_range,
        ),
        CompletionContext::FreshRelationshipSource => {
            fresh_relationship_source_items(document, snapshot, workspace_facts, prefix, edit_range)
        }
        CompletionContext::Suppress => Vec::new(),
    }
}

fn style_property_items(kind: StyleBlockKind) -> Vec<(&'static str, &'static str)> {
    let mut items = SHARED_STYLE_PROPERTY_ITEMS.to_vec();

    match kind {
        StyleBlockKind::Element => items.extend_from_slice(ELEMENT_STYLE_PROPERTY_ITEMS),
        StyleBlockKind::Relationship => items.extend_from_slice(RELATIONSHIP_STYLE_PROPERTY_ITEMS),
    }

    items
}

fn style_value_completion_items(
    context: StyleValueCompletionContext,
    edit_range: Option<Range>,
    prefix: &str,
) -> Vec<CompletionItem> {
    if context.hash_prefixed {
        return Vec::new();
    }

    context
        .kind
        .candidates()
        .iter()
        .copied()
        .filter(|candidate| style_value_prefix_matches(candidate, prefix))
        .map(|candidate| CompletionItem {
            label: candidate.to_owned(),
            kind: Some(context.kind.completion_kind()),
            detail: Some(context.kind.detail().to_owned()),
            text_edit: edit_range.map(|range| {
                CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: candidate.to_owned(),
                })
            }),
            ..CompletionItem::default()
        })
        .collect()
}

fn completion_context(
    text: &str,
    snapshot: &DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> CompletionContext {
    let style_context = style_block_context(text, snapshot, offset, prefix_start);

    // Incomplete quoted text usually only exists through parser recovery, so do
    // not rely on syntax nodes alone to suppress noisy identifier/keyword
    // completions while the user is typing inside a string.
    if is_inside_quoted_string(text, offset) {
        return style_context
            .filter(CompletionContext::allows_quoted_completion)
            .unwrap_or(CompletionContext::Suppress);
    }

    // First keep the existing syntax-backed refinements intact. Only after those
    // do we attempt the new semantic relationship-endpoint completion surface.
    style_context
        .or_else(|| relationship_completion_context(text, snapshot, offset, prefix_start))
        .or_else(|| fresh_relationship_source_context(text, snapshot, offset, prefix_start))
        .unwrap_or(CompletionContext::FixedVocabulary)
}

fn style_block_context(
    text: &str,
    snapshot: &DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> Option<CompletionContext> {
    syntax_nodes_at_offset(text, snapshot, offset, prefix_start)
        .into_iter()
        .find_map(|node| style_block_context_from_node(text, node, offset, prefix_start))
}

fn relationship_completion_context(
    text: &str,
    snapshot: &DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> Option<CompletionContext> {
    syntax_nodes_at_offset(text, snapshot, offset, prefix_start)
        .into_iter()
        .find_map(|node| {
            relationship_completion_context_from_node(text, node, offset, prefix_start)
        })
}

fn fresh_relationship_source_context(
    text: &str,
    snapshot: &DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> Option<CompletionContext> {
    if !is_fresh_relationship_source_insertion_point(text, offset, prefix_start) {
        return None;
    }

    syntax_nodes_at_offset(text, snapshot, offset, prefix_start)
        .into_iter()
        .find_map(fresh_relationship_source_context_from_node)
}

fn style_block_context_from_node(
    text: &str,
    node: tree_sitter::Node<'_>,
    offset: usize,
    prefix_start: usize,
) -> Option<CompletionContext> {
    let mut current = node;
    let mut style_setting = None;
    let mut style_rule_block = None;
    let mut inside_properties_block = false;
    let style_kind = loop {
        match current.kind() {
            "style_setting" if style_setting.is_none() => style_setting = Some(current),
            "properties_block" => inside_properties_block = true,
            "style_rule_block" if style_rule_block.is_none() => style_rule_block = Some(current),
            "element_style" => break Some(StyleBlockKind::Element),
            "relationship_style" => break Some(StyleBlockKind::Relationship),
            _ => {}
        }

        current = current.parent()?;
    }?;

    if inside_properties_block {
        return Some(CompletionContext::Suppress);
    }

    if let Some(style_setting) = style_setting {
        let name = style_setting.child_by_field_name("name")?;

        // Once the cursor has moved off the property name and into the value,
        // suppress the global keyword list as well. That keeps `metadata true`
        // or `routing Orthogonal` from showing unrelated workspace keywords.
        return Some(if style_kind == StyleBlockKind::Element {
            element_style_completion_context(text, offset, prefix_start, Some(name))
        } else if node_matches_cursor(name, offset, prefix_start) {
            CompletionContext::StyleProperties(StyleBlockKind::Relationship)
        } else {
            CompletionContext::Suppress
        });
    }

    let style_rule_block = style_rule_block?;
    if !span_contains(
        style_rule_block.start_byte(),
        style_rule_block.end_byte(),
        offset,
    ) {
        return None;
    }

    Some(if style_kind == StyleBlockKind::Element {
        element_style_completion_context(text, offset, prefix_start, None)
    } else if is_style_property_insertion_point(text, prefix_start) {
        CompletionContext::StyleProperties(StyleBlockKind::Relationship)
    } else {
        CompletionContext::Suppress
    })
}

fn element_style_completion_context(
    text: &str,
    offset: usize,
    prefix_start: usize,
    property_name: Option<tree_sitter::Node<'_>>,
) -> CompletionContext {
    element_style_value_context(text, offset, prefix_start).map_or_else(
        || {
            if property_name.is_some_and(|name| node_matches_cursor(name, offset, prefix_start))
                || property_name.is_none() && is_style_property_insertion_point(text, prefix_start)
            {
                CompletionContext::StyleProperties(StyleBlockKind::Element)
            } else {
                CompletionContext::Suppress
            }
        },
        CompletionContext::StyleValues,
    )
}

fn element_style_value_context(
    text: &str,
    offset: usize,
    prefix_start: usize,
) -> Option<StyleValueCompletionContext> {
    let safe_offset = offset.min(text.len());
    let safe_prefix_start = prefix_start.min(text.len());
    let line_start = text[..safe_prefix_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let indent_len = text[line_start..safe_offset]
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let property_start = line_start + indent_len;
    let property_end = property_start
        + text[property_start..safe_offset]
            .bytes()
            .take_while(
                |byte| matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'.' | b'-'),
            )
            .count();
    if property_end == property_start || safe_prefix_start <= property_end {
        return None;
    }

    // Partially typed style values recover as ERROR nodes before Tree-sitter can
    // rebuild a stable `style_setting`, so once we know we are inside an element
    // style block we recover the property/value split from the current line.
    let kind = bounded_element_style_value_kind(&text[property_start..property_end])?;
    let quoted = is_inside_quoted_string(text, offset);
    if quoted && !kind.supports_quoted_values() {
        return None;
    }

    let prefix_mode = style_value_prefix_mode(kind, text, property_end, prefix_start)?;
    Some(StyleValueCompletionContext {
        kind,
        hash_prefixed: prefix_mode == StyleValuePrefixMode::HashPrefixed,
    })
}

fn bounded_element_style_value_kind(property_name: &str) -> Option<StyleValueKind> {
    if COLOR_STYLE_VALUE_PROPERTIES.contains(&property_name) {
        Some(StyleValueKind::NamedColor)
    } else if BOOLEAN_STYLE_VALUE_PROPERTIES.contains(&property_name) {
        Some(StyleValueKind::Boolean)
    } else if BORDER_STYLE_VALUE_PROPERTIES.contains(&property_name) {
        Some(StyleValueKind::Border)
    } else if SHAPE_STYLE_VALUE_PROPERTIES.contains(&property_name) {
        Some(StyleValueKind::Shape)
    } else {
        None
    }
}

fn style_value_prefix_mode(
    kind: StyleValueKind,
    text: &str,
    name_end: usize,
    prefix_start: usize,
) -> Option<StyleValuePrefixMode> {
    let safe_name_end = name_end.min(text.len());
    let safe_prefix_start = prefix_start.min(text.len());
    if safe_prefix_start < safe_name_end {
        return None;
    }

    let leading = &text[safe_name_end..safe_prefix_start];
    if leading.contains('\n') {
        return None;
    }

    let trimmed = leading.trim_start_matches(|char: char| char.is_ascii_whitespace());
    match kind {
        StyleValueKind::NamedColor => named_color_prefix_mode(trimmed),
        _ if kind.supports_quoted_values() => {
            matches!(trimmed, "" | "\"").then_some(StyleValuePrefixMode::Plain)
        }
        _ => trimmed.is_empty().then_some(StyleValuePrefixMode::Plain),
    }
}

fn named_color_prefix_mode(trimmed: &str) -> Option<StyleValuePrefixMode> {
    if trimmed.is_empty() {
        Some(StyleValuePrefixMode::Plain)
    } else if let Some(rest) = trimmed.strip_prefix('#') {
        rest.chars()
            .all(|char| !char.is_ascii_whitespace())
            .then_some(StyleValuePrefixMode::HashPrefixed)
    } else {
        None
    }
}

fn style_value_prefix_matches(candidate: &str, prefix: &str) -> bool {
    prefix.is_empty()
        || candidate
            .get(..prefix.len())
            .is_some_and(|leading| leading.eq_ignore_ascii_case(prefix))
}

fn relationship_completion_context_from_node(
    text: &str,
    node: tree_sitter::Node<'_>,
    offset: usize,
    prefix_start: usize,
) -> Option<CompletionContext> {
    let mut current = node;

    loop {
        match current.kind() {
            "dynamic_relationship" => {
                return relationship_node_matches_cursor(current, offset, prefix_start)
                    .then_some(CompletionContext::Suppress);
            }
            "relationship" => {
                if is_deployment_relationship(current) {
                    return Some(CompletionContext::Suppress);
                }

                return if relationship_node_matches_cursor(current, offset, prefix_start) {
                    Some(
                        relationship_endpoint_context(text, current, offset, prefix_start)
                            .unwrap_or(CompletionContext::Suppress),
                    )
                } else {
                    relationship_endpoint_context(text, current, offset, prefix_start)
                };
            }
            _ => {}
        }

        current = current.parent()?;
    }
}

fn relationship_node_matches_cursor(
    relationship: tree_sitter::Node<'_>,
    offset: usize,
    prefix_start: usize,
) -> bool {
    let cursor_probe = offset;
    let prefix_probe = prefix_start.saturating_sub(1);

    span_contains(
        relationship.start_byte(),
        relationship.end_byte(),
        cursor_probe,
    ) || span_contains(
        relationship.start_byte(),
        relationship.end_byte(),
        prefix_probe,
    ) || cursor_probe == relationship.end_byte()
}

fn relationship_endpoint_context(
    text: &str,
    relationship: tree_sitter::Node<'_>,
    offset: usize,
    prefix_start: usize,
) -> Option<CompletionContext> {
    let safe_offset = offset.min(text.len());
    let safe_prefix_start = prefix_start.min(text.len());
    let operator = relationship.child_by_field_name("operator")?;
    let source = relationship.child_by_field_name("source");
    let destination = relationship.child_by_field_name("destination");

    if source.is_some_and(|source| node_matches_cursor(source, offset, prefix_start)) {
        return Some(CompletionContext::RelationshipIdentifier(
            RelationshipCompletionContext {
                endpoint: RelationshipEndpoint::Source,
                source_text: None,
            },
        ));
    }

    // Tree-sitter still gives us a stable relationship node when the source has
    // not been typed yet, so treat the blank span before the operator as a
    // source insertion point too.
    if source.is_none()
        && safe_prefix_start <= operator.start_byte()
        && is_blank_endpoint_insertion_point(text, relationship.start_byte(), safe_offset)
    {
        return Some(CompletionContext::RelationshipIdentifier(
            RelationshipCompletionContext {
                endpoint: RelationshipEndpoint::Source,
                source_text: None,
            },
        ));
    }

    if destination.is_some_and(|destination| node_matches_cursor(destination, offset, prefix_start))
    {
        return Some(CompletionContext::RelationshipIdentifier(
            RelationshipCompletionContext {
                endpoint: RelationshipEndpoint::Destination,
                source_text: source
                    .filter(|source| source.kind() == "identifier")
                    .map(|source| node_text(source, text)),
            },
        ));
    }

    // Likewise, a blank destination currently appears as either a zero-width
    // identifier or as an empty segment after the operator. Treat that as a
    // destination insertion point so completion stays useful before any letters
    // have been typed.
    if safe_prefix_start >= operator.end_byte()
        && destination.is_none_or(|destination| destination.start_byte() == destination.end_byte())
        && is_blank_endpoint_insertion_point(text, operator.end_byte(), safe_offset)
    {
        return Some(CompletionContext::RelationshipIdentifier(
            RelationshipCompletionContext {
                endpoint: RelationshipEndpoint::Destination,
                source_text: source
                    .filter(|source| source.kind() == "identifier")
                    .map(|source| node_text(source, text)),
            },
        ));
    }

    None
}

fn node_matches_cursor(node: tree_sitter::Node<'_>, offset: usize, prefix_start: usize) -> bool {
    let cursor_probe = offset;
    let prefix_probe = prefix_start.saturating_sub(1);

    span_contains(node.start_byte(), node.end_byte(), cursor_probe)
        || span_contains(node.start_byte(), node.end_byte(), prefix_probe)
        || cursor_probe == node.end_byte()
}

fn is_blank_endpoint_insertion_point(text: &str, start_byte: usize, offset: usize) -> bool {
    let safe_start = start_byte.min(text.len());
    let safe_offset = offset.min(text.len());
    if safe_offset < safe_start {
        return false;
    }

    text[safe_start..safe_offset].trim().is_empty()
}

fn is_deployment_relationship(node: tree_sitter::Node<'_>) -> bool {
    let mut current = node;

    while let Some(parent) = current.parent() {
        match parent.kind() {
            "deployment_environment"
            | "deployment_environment_block"
            | "deployment_node"
            | "deployment_node_block"
            | "infrastructure_node"
            | "container_instance"
            | "software_system_instance"
            | "deployment_instance_block" => return true,
            _ => current = parent,
        }
    }

    false
}

fn fresh_relationship_source_context_from_node(
    node: tree_sitter::Node<'_>,
) -> Option<CompletionContext> {
    if is_deployment_relationship(node) {
        return None;
    }

    let mut current = node;
    loop {
        if current.kind().ends_with("_block") {
            return fresh_relationship_source_block_kind(current.kind())
                .then_some(CompletionContext::FreshRelationshipSource);
        }

        current = current.parent()?;
    }
}

fn fresh_relationship_source_block_kind(kind: &str) -> bool {
    matches!(
        kind,
        "model_block"
            | "person_block"
            | "software_system_block"
            | "container_block"
            | "component_block"
            | "group_block"
            | "enterprise_block"
            | "custom_element_block"
            | "element_directive_block"
            | "elements_block"
    )
}

fn is_fresh_relationship_source_insertion_point(
    text: &str,
    offset: usize,
    prefix_start: usize,
) -> bool {
    let safe_offset = offset.min(text.len());
    let safe_prefix_start = prefix_start.min(text.len());
    if safe_prefix_start > safe_offset {
        return false;
    }

    let line_start = text[..safe_prefix_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let line_end = text[safe_offset..]
        .find('\n')
        .map_or(text.len(), |index| safe_offset + index);
    let prefix = &text[safe_prefix_start..safe_offset];

    !prefix.starts_with('!')
        && text[line_start..safe_prefix_start].trim().is_empty()
        && text[safe_offset..line_end].trim().is_empty()
}

// =============================================================================
// Semantic relationship completion
// =============================================================================

fn fresh_relationship_source_items(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
    prefix: &str,
    edit_range: Option<Range>,
) -> Vec<CompletionItem> {
    let mut items = relationship_identifier_items(
        document,
        snapshot,
        workspace_facts,
        &RelationshipCompletionContext {
            endpoint: RelationshipEndpoint::Source,
            source_text: None,
        },
        prefix,
        edit_range,
    );
    items.extend(keyword_completion_items(
        FIXED_COMPLETION_ITEMS.to_vec(),
        edit_range,
        prefix,
    ));
    items
}

fn relationship_identifier_items(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
    context: &RelationshipCompletionContext,
    prefix: &str,
    edit_range: Option<Range>,
) -> Vec<CompletionItem> {
    let candidates =
        match workspace_relationship_completion_candidates(document, workspace_facts, context) {
            WorkspaceCompletionOutcome::Candidates(candidates) => candidates,
            WorkspaceCompletionOutcome::NoWorkspaceContext => {
                same_document_relationship_completion_candidates(snapshot, context)
            }
            WorkspaceCompletionOutcome::Suppress => return Vec::new(),
        };

    candidates
        .into_values()
        .filter(|candidate| prefix.is_empty() || candidate.label.starts_with(prefix))
        .map(|candidate| CompletionItem {
            label: candidate.label.clone(),
            kind: Some(CompletionItemKind::REFERENCE),
            detail: Some(candidate.detail),
            text_edit: edit_range.map(|range| {
                CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: candidate.label,
                })
            }),
            ..CompletionItem::default()
        })
        .collect()
}

fn workspace_relationship_completion_candidates(
    document: &DocumentState,
    workspace_facts: Option<&WorkspaceFacts>,
    context: &RelationshipCompletionContext,
) -> WorkspaceCompletionOutcome {
    let Some(workspace_facts) = workspace_facts else {
        return WorkspaceCompletionOutcome::NoWorkspaceContext;
    };
    let Some(document_id) = workspace_document_id(document) else {
        return WorkspaceCompletionOutcome::NoWorkspaceContext;
    };

    let candidate_instances = workspace_facts
        .candidate_instances_for(&document_id)
        .copied()
        .collect::<Vec<_>>();
    if candidate_instances.is_empty() {
        return WorkspaceCompletionOutcome::NoWorkspaceContext;
    }

    // The first rollout stays flat-mode only. If any candidate workspace instance
    // would require hierarchical element keys for this document, prefer no answer
    // over suggesting identifiers the user cannot safely insert.
    if candidate_instances.iter().any(|instance_id| {
        workspace_facts
            .workspace_index(*instance_id)
            .and_then(|workspace_index| workspace_index.element_identifier_mode_for(&document_id))
            == Some(ElementIdentifierMode::Hierarchical)
    }) {
        return WorkspaceCompletionOutcome::Suppress;
    }

    let allowed_kinds = match context.endpoint {
        RelationshipEndpoint::Source => CORE_ELEMENT_KINDS,
        RelationshipEndpoint::Destination => {
            let Some(source_kind) = unanimous_workspace_source_kind(
                workspace_facts,
                &candidate_instances,
                context.source_text.as_deref(),
            ) else {
                return WorkspaceCompletionOutcome::Suppress;
            };
            allowed_destination_kinds(source_kind)
        }
    };

    unanimous_workspace_candidate_map(workspace_facts, &candidate_instances, allowed_kinds).map_or(
        WorkspaceCompletionOutcome::Suppress,
        WorkspaceCompletionOutcome::Candidates,
    )
}

fn same_document_relationship_completion_candidates(
    snapshot: &DocumentSnapshot,
    context: &RelationshipCompletionContext,
) -> BTreeMap<String, IdentifierCompletionCandidate> {
    if !snapshot_uses_flat_identifier_mode(snapshot) {
        return BTreeMap::new();
    }

    let allowed_kinds = match context.endpoint {
        RelationshipEndpoint::Source => CORE_ELEMENT_KINDS,
        RelationshipEndpoint::Destination => {
            let Some(source_kind) =
                same_document_source_kind(snapshot, context.source_text.as_deref())
            else {
                return BTreeMap::new();
            };
            allowed_destination_kinds(source_kind)
        }
    };

    candidate_map_from_symbols(snapshot.symbols(), allowed_kinds)
}

fn unanimous_workspace_source_kind(
    workspace_facts: &WorkspaceFacts,
    candidate_instances: &[WorkspaceInstanceId],
    source_text: Option<&str>,
) -> Option<SymbolKind> {
    let source_text = source_text?;
    let mut resolved_kind = None;

    for instance_id in candidate_instances {
        let workspace_index = workspace_facts.workspace_index(*instance_id)?;
        if workspace_index
            .duplicate_element_bindings()
            .contains_key(source_text)
        {
            return None;
        }

        let handle = workspace_index.unique_element_bindings().get(source_text)?;
        let symbol = workspace_symbol(workspace_facts, handle)?;
        if !is_core_element_kind(symbol.kind) {
            return None;
        }

        if resolved_kind
            .as_ref()
            .is_some_and(|existing| existing != &symbol.kind)
        {
            return None;
        }

        resolved_kind = Some(symbol.kind);
    }

    resolved_kind
}

fn unanimous_workspace_candidate_map(
    workspace_facts: &WorkspaceFacts,
    candidate_instances: &[WorkspaceInstanceId],
    allowed_kinds: &[SymbolKind],
) -> Option<BTreeMap<String, IdentifierCompletionCandidate>> {
    let mut candidates = None;

    for instance_id in candidate_instances {
        let workspace_index = workspace_facts.workspace_index(*instance_id)?;
        let current =
            candidate_map_from_workspace_index(workspace_facts, workspace_index, allowed_kinds)?;
        if candidates
            .as_ref()
            .is_some_and(|existing| existing != &current)
        {
            return None;
        }
        candidates = Some(current);
    }

    candidates
}

fn candidate_map_from_workspace_index(
    workspace_facts: &WorkspaceFacts,
    workspace_index: &WorkspaceIndex,
    allowed_kinds: &[SymbolKind],
) -> Option<BTreeMap<String, IdentifierCompletionCandidate>> {
    let mut candidates = BTreeMap::new();

    for (binding, handle) in workspace_index.unique_element_bindings() {
        let symbol = workspace_symbol(workspace_facts, handle)?;
        if allowed_kinds.contains(&symbol.kind) {
            candidates.insert(
                binding.clone(),
                completion_candidate(binding.clone(), symbol),
            );
        }
    }

    Some(candidates)
}

fn candidate_map_from_symbols(
    symbols: &[Symbol],
    allowed_kinds: &[SymbolKind],
) -> BTreeMap<String, IdentifierCompletionCandidate> {
    let mut unique = BTreeMap::new();
    let mut duplicates = std::collections::BTreeSet::new();

    for symbol in symbols {
        let Some(binding_name) = symbol.binding_name.as_deref() else {
            continue;
        };
        if !allowed_kinds.contains(&symbol.kind) {
            continue;
        }

        if duplicates.contains(binding_name) {
            continue;
        }

        let candidate = completion_candidate(binding_name.to_owned(), symbol);
        if unique.insert(binding_name.to_owned(), candidate).is_some() {
            unique.remove(binding_name);
            duplicates.insert(binding_name.to_owned());
        }
    }

    unique
}

fn completion_candidate(label: String, symbol: &Symbol) -> IdentifierCompletionCandidate {
    IdentifierCompletionCandidate {
        label,
        detail: completion_candidate_detail(symbol),
    }
}

fn completion_candidate_detail(symbol: &Symbol) -> String {
    let kind = symbol_kind_label(symbol.kind);
    let binding_name = symbol.binding_name.as_deref().unwrap_or_default();
    if symbol.display_name == binding_name {
        format!("{kind} identifier")
    } else {
        format!("{kind}: {}", symbol.display_name)
    }
}

fn same_document_source_kind(
    snapshot: &DocumentSnapshot,
    source_text: Option<&str>,
) -> Option<SymbolKind> {
    let source_text = source_text?;
    let mut matches = snapshot.symbols().iter().filter(|symbol| {
        is_core_element_kind(symbol.kind) && symbol.binding_name.as_deref() == Some(source_text)
    });
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    Some(first.kind)
}

fn snapshot_uses_flat_identifier_mode(snapshot: &DocumentSnapshot) -> bool {
    matches!(
        snapshot_model_identifier_mode(snapshot)
            .or_else(|| snapshot_workspace_identifier_mode(snapshot)),
        Some(IdentifierMode::Flat) | None
    )
}

fn snapshot_model_identifier_mode(snapshot: &DocumentSnapshot) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(snapshot, &DirectiveContainer::Model)
}

fn snapshot_workspace_identifier_mode(snapshot: &DocumentSnapshot) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(snapshot, &DirectiveContainer::Workspace)
}

fn last_identifier_mode_for_container(
    snapshot: &DocumentSnapshot,
    container: &DirectiveContainer,
) -> Option<IdentifierMode> {
    snapshot
        .identifier_modes()
        .iter()
        .rev()
        .find(|fact| fact.container == *container)
        .map(|fact| fact.mode.clone())
}

fn workspace_document_id(document: &DocumentState) -> Option<DocumentId> {
    document.workspace_document_id().cloned()
}

fn workspace_symbol<'a>(
    workspace_facts: &'a WorkspaceFacts,
    handle: &SymbolHandle,
) -> Option<&'a Symbol> {
    workspace_facts
        .document(handle.document())?
        .snapshot()
        .symbols()
        .get(handle.symbol_id().0)
}

const fn allowed_destination_kinds(source_kind: SymbolKind) -> &'static [SymbolKind] {
    match source_kind {
        SymbolKind::Person
        | SymbolKind::SoftwareSystem
        | SymbolKind::Container
        | SymbolKind::Component => CORE_ELEMENT_KINDS,
        _ => &[],
    }
}

const fn is_core_element_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Person
            | SymbolKind::SoftwareSystem
            | SymbolKind::Container
            | SymbolKind::Component
    )
}

const fn symbol_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Person => "Person",
        SymbolKind::SoftwareSystem => "Software system",
        SymbolKind::Container => "Container",
        SymbolKind::Component => "Component",
        SymbolKind::DeploymentNode => "Deployment node",
        SymbolKind::InfrastructureNode => "Infrastructure node",
        SymbolKind::ContainerInstance => "Container instance",
        SymbolKind::SoftwareSystemInstance => "Software system instance",
        SymbolKind::Relationship => "Relationship",
    }
}

// =============================================================================
// Generic syntax helpers
// =============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuoteScanState {
    Code,
    LineComment,
    BlockComment,
    String,
    TextBlockString,
}

fn is_inside_quoted_string(text: &str, offset: usize) -> bool {
    let safe_offset = offset.min(text.len());
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut state = QuoteScanState::Code;

    while index < safe_offset {
        state = match state {
            QuoteScanState::Code => {
                if let Some(prefix_len) = line_comment_prefix_len(bytes, index) {
                    index += prefix_len;
                    QuoteScanState::LineComment
                } else if bytes_at(bytes, index, b"/*") {
                    index += 2;
                    QuoteScanState::BlockComment
                } else if bytes_at(bytes, index, b"\"\"\"") {
                    index += 3;
                    QuoteScanState::TextBlockString
                } else if bytes[index] == b'"' {
                    index += 1;
                    QuoteScanState::String
                } else {
                    index += 1;
                    QuoteScanState::Code
                }
            }
            QuoteScanState::LineComment => {
                if bytes[index] == b'\n' {
                    index += 1;
                    QuoteScanState::Code
                } else {
                    index += 1;
                    QuoteScanState::LineComment
                }
            }
            QuoteScanState::BlockComment => {
                if bytes_at(bytes, index, b"*/") {
                    index += 2;
                    QuoteScanState::Code
                } else {
                    index += 1;
                    QuoteScanState::BlockComment
                }
            }
            QuoteScanState::String => {
                if bytes[index] == b'\\' {
                    index = skip_string_escape(bytes, index, safe_offset);
                    QuoteScanState::String
                } else if bytes[index] == b'"' {
                    index += 1;
                    QuoteScanState::Code
                } else {
                    index += 1;
                    QuoteScanState::String
                }
            }
            QuoteScanState::TextBlockString => {
                if bytes_at(bytes, index, b"\"\"\"") {
                    index += 3;
                    QuoteScanState::Code
                } else {
                    index += 1;
                    QuoteScanState::TextBlockString
                }
            }
        };
    }

    matches!(
        state,
        QuoteScanState::String | QuoteScanState::TextBlockString
    )
}

fn bytes_at(bytes: &[u8], index: usize, needle: &[u8]) -> bool {
    bytes.get(index..index + needle.len()) == Some(needle)
}

fn line_comment_prefix_len(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes_at(bytes, index, b"//")
        || (bytes.get(index) == Some(&b'#') && matches!(bytes.get(index + 1), Some(b' ' | b'\t')))
    {
        Some(2)
    } else {
        None
    }
}

fn skip_string_escape(bytes: &[u8], index: usize, limit: usize) -> usize {
    let mut next = index + 1;
    if next >= limit {
        return limit;
    }

    match bytes[next] {
        b'\r' => {
            next += 1;
            if next < limit && bytes[next] == b'\n' {
                next += 1;
            }
            while next < limit && matches!(bytes[next], b' ' | b'\t') {
                next += 1;
            }
            next
        }
        b'\n' => {
            next += 1;
            while next < limit && matches!(bytes[next], b' ' | b'\t') {
                next += 1;
            }
            next
        }
        _ => (index + 2).min(limit),
    }
}

fn syntax_nodes_at_offset<'a>(
    text: &str,
    snapshot: &'a DocumentSnapshot,
    offset: usize,
    prefix_start: usize,
) -> Vec<tree_sitter::Node<'a>> {
    if text.is_empty() {
        return Vec::new();
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
        previous_non_whitespace_probe(text, offset)
            .unwrap_or(0)
            .min(last_byte),
        previous_non_whitespace_probe(text, prefix_start)
            .unwrap_or(0)
            .min(last_byte),
    ];

    let mut nodes = Vec::new();
    for probe in probes {
        let probe_end = (probe + 1).min(text.len());
        let Some(node) = root
            .descendant_for_byte_range(probe, probe_end)
            .or_else(|| root.descendant_for_byte_range(probe, probe))
        else {
            continue;
        };

        if nodes.iter().any(|existing: &tree_sitter::Node<'_>| {
            existing.start_byte() == node.start_byte()
                && existing.end_byte() == node.end_byte()
                && existing.kind() == node.kind()
        }) {
            continue;
        }

        nodes.push(node);
    }

    nodes
}

fn previous_non_whitespace_probe(text: &str, offset: usize) -> Option<usize> {
    let safe_offset = offset.min(text.len());
    text.as_bytes()[..safe_offset]
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
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

fn keyword_completion_items(
    candidates: Vec<(&'static str, &'static str)>,
    edit_range: Option<Range>,
    prefix: &str,
) -> Vec<CompletionItem> {
    candidates
        .into_iter()
        .filter(|(label, _)| prefix.is_empty() || label.starts_with(prefix))
        .map(|(label, detail)| keyword_completion_item(label, detail, edit_range))
        .collect()
}

fn keyword_completion_item(label: &str, detail: &str, edit_range: Option<Range>) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_owned()),
        text_edit: edit_range.map(|range| {
            CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: label.to_owned(),
            })
        }),
        ..CompletionItem::default()
    }
}

fn node_text(node: tree_sitter::Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .expect("node text should be utf-8")
        .to_owned()
}

fn completion_prefix_start(text: &str, offset: usize) -> usize {
    let safe_offset = offset.min(text.len());
    let bytes = text.as_bytes();

    bytes[..safe_offset]
        .iter()
        .rposition(|byte| {
            !matches!(
                byte,
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'!' | b'_' | b'.' | b'-'
            )
        })
        .map_or(0, |index| index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAMMAR_JS: &str = include_str!("../../../structurizr-grammar/grammar.js");

    #[test]
    fn bounded_style_value_tables_match_grammar_js() {
        assert_eq!(
            NAMED_COLOR_VALUES.to_vec(),
            extract_js_string_array(GRAMMAR_JS, "NAMED_COLORS")
        );
        assert_eq!(
            COLOR_STYLE_VALUE_PROPERTIES.to_vec(),
            extract_js_string_array(GRAMMAR_JS, "COLOR_STYLE_PROPERTIES")
        );
        assert_eq!(
            BOOLEAN_STYLE_VALUE_VALUES.to_vec(),
            extract_js_string_array(GRAMMAR_JS, "BOOLEAN_VALUES")
        );
        assert_eq!(
            SHAPE_STYLE_VALUE_VALUES.to_vec(),
            extract_js_string_array(GRAMMAR_JS, "SHAPE_VALUES")
        );
        assert_eq!(
            BORDER_STYLE_VALUE_VALUES.to_vec(),
            extract_js_string_array(GRAMMAR_JS, "BORDER_VALUES")
        );
    }

    #[test]
    fn bounded_style_value_property_groups_match_grammar_js() {
        let style_setting = extract_style_setting_section(GRAMMAR_JS);
        assert!(
            style_setting.contains("alias(\"shape\", $.identifier)"),
            "style_setting should still model the shape property explicitly"
        );
        assert!(
            style_setting.contains("alias(\"border\", $.identifier)"),
            "style_setting should still model the border property explicitly"
        );

        let boolean_properties = extract_double_quoted_strings(
            style_setting
                .lines()
                .find(|line| line.contains("choice(\"metadata\""))
                .expect("style_setting boolean choice should exist"),
        );
        for property in BOOLEAN_STYLE_VALUE_PROPERTIES {
            assert!(
                boolean_properties.contains(property),
                "style_setting boolean choice should still include `{property}`"
            );
        }
        for property in ["dashed", "jump"] {
            assert!(
                boolean_properties.contains(&property),
                "style_setting boolean choice should still include `{property}`"
            );
            assert_eq!(
                bounded_element_style_value_kind(property),
                None,
                "`{property}` should stay out of element-style value completion"
            );
        }
    }

    #[test]
    fn bounded_element_style_value_kind_matches_supported_properties() {
        for property in COLOR_STYLE_VALUE_PROPERTIES {
            assert_eq!(
                bounded_element_style_value_kind(property),
                Some(StyleValueKind::NamedColor),
                "`{property}` should map to named-color completion"
            );
        }
        for property in BOOLEAN_STYLE_VALUE_PROPERTIES {
            assert_eq!(
                bounded_element_style_value_kind(property),
                Some(StyleValueKind::Boolean),
                "`{property}` should map to boolean completion"
            );
        }
        for property in BORDER_STYLE_VALUE_PROPERTIES {
            assert_eq!(
                bounded_element_style_value_kind(property),
                Some(StyleValueKind::Border),
                "`{property}` should map to border completion"
            );
        }
        for property in SHAPE_STYLE_VALUE_PROPERTIES {
            assert_eq!(
                bounded_element_style_value_kind(property),
                Some(StyleValueKind::Shape),
                "`{property}` should map to shape completion"
            );
        }
    }

    fn extract_style_setting_section(source: &str) -> &str {
        let marker = "style_setting: ($) =>";
        let start = source
            .find(marker)
            .expect("grammar should define style_setting");
        let rest = &source[start..];
        let end = rest
            .find("_style_value: ($) =>")
            .expect("style_setting section should end before _style_value");
        &rest[..end]
    }

    fn extract_js_string_array<'a>(source: &'a str, const_name: &str) -> Vec<&'a str> {
        let marker = format!("const {const_name} = [");
        let start = source
            .find(&marker)
            .unwrap_or_else(|| panic!("grammar should define {const_name}"))
            + marker.len();
        let rest = &source[start..];
        let end = rest
            .find("];")
            .unwrap_or_else(|| panic!("{const_name} array should terminate"));
        extract_double_quoted_strings(&rest[..end])
    }

    fn extract_double_quoted_strings(input: &str) -> Vec<&str> {
        let bytes = input.as_bytes();
        let mut index = 0;
        let mut values = Vec::new();

        while index < bytes.len() {
            if bytes[index] != b'"' {
                index += 1;
                continue;
            }

            let start = index + 1;
            index += 1;
            while index < bytes.len() && bytes[index] != b'"' {
                index += 1;
            }
            assert!(index < bytes.len(), "quoted string should terminate");
            values.push(&input[start..index]);
            index += 1;
        }

        values
    }
}
