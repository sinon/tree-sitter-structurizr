//! Bounded-MVP symbol, reference, and identifier-mode facts.

use crate::includes::{DirectiveContainer, DirectiveValueKind};
use crate::span::TextSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Person,
    SoftwareSystem,
    Container,
    Component,
    Relationship,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub display_name: String,
    pub binding_name: Option<String>,
    pub span: TextSpan,
    pub parent: Option<SymbolId>,
    pub syntax_node_kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    RelationshipSource,
    RelationshipDestination,
    ViewScope,
    ViewInclude,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceTargetHint {
    Element,
    Relationship,
    ElementOrRelationship,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub kind: ReferenceKind,
    pub raw_text: String,
    pub span: TextSpan,
    pub target_hint: ReferenceTargetHint,
    pub container_node_kind: String,
    pub containing_symbol: Option<SymbolId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierMode {
    Flat,
    Hierarchical,
    Other(String),
}

impl IdentifierMode {
    pub(crate) fn from_raw(raw_value: &str) -> Self {
        match raw_value.to_ascii_lowercase().as_str() {
            "flat" => Self::Flat,
            "hierarchical" => Self::Hierarchical,
            _ => Self::Other(raw_value.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierModeFact {
    pub mode: IdentifierMode,
    pub raw_value: String,
    pub value_kind: DirectiveValueKind,
    pub span: TextSpan,
    pub value_span: TextSpan,
    pub container: DirectiveContainer,
}
