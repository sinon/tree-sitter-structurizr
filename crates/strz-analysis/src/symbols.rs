//! Bounded-MVP symbol, reference, and identifier-mode facts.

use std::fmt;

use crate::includes::{DirectiveContainer, DirectiveValueKind};
use crate::span::TextSpan;

/// Stable index assigned to an extracted symbol within one document snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub usize);

/// High-level declaration kinds extracted from Structurizr DSL nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// A `person` declaration.
    Person,
    /// A `softwareSystem` declaration.
    SoftwareSystem,
    /// A `container` declaration.
    Container,
    /// A `component` declaration.
    Component,
    /// A `deploymentEnvironment` declaration with a binding identifier.
    DeploymentEnvironment,
    /// A `deploymentNode` declaration with a binding identifier.
    DeploymentNode,
    /// An `infrastructureNode` declaration with a binding identifier.
    InfrastructureNode,
    /// A `containerInstance` declaration with a binding identifier.
    ContainerInstance,
    /// A `softwareSystemInstance` declaration with a binding identifier.
    SoftwareSystemInstance,
    /// A relationship declaration with its own identifier.
    Relationship,
}

/// Describes one declaration symbol extracted from a document.
#[derive(Clone, PartialEq, Eq)]
pub struct Symbol {
    /// Snapshot-local identifier assigned during extraction.
    pub id: SymbolId,
    /// High-level kind of declaration that produced the symbol.
    pub kind: SymbolKind,
    /// User-facing display label inferred from the declaration.
    pub display_name: String,
    /// Bound identifier, if the declaration introduces one.
    pub binding_name: Option<String>,
    /// Span of the bound identifier token, if the declaration introduces one.
    pub binding_span: Option<TextSpan>,
    /// Source-derived description text for hover and other read-only UX.
    pub description: Option<String>,
    /// Source-derived technology text for hover and other read-only UX.
    pub technology: Option<String>,
    /// Source-derived tags collected from declaration headers and bodies.
    pub tags: Vec<String>,
    /// Source-derived URL text for hover and other read-only UX.
    pub url: Option<String>,
    /// Span of the full declaration node.
    pub span: TextSpan,
    /// Nearest enclosing declaration symbol, if one exists.
    pub parent: Option<SymbolId>,
    /// Exact Tree-sitter node kind that produced the symbol.
    pub syntax_node_kind: String,
}

impl fmt::Debug for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("Symbol");
        debug
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("display_name", &self.display_name)
            .field("binding_name", &self.binding_name);
        if self.binding_span.is_some() {
            debug.field("binding_span", &self.binding_span);
        }

        if self.description.is_some() {
            debug.field("description", &self.description);
        }
        if self.technology.is_some() {
            debug.field("technology", &self.technology);
        }
        if !self.tags.is_empty() {
            debug.field("tags", &self.tags);
        }
        if self.url.is_some() {
            debug.field("url", &self.url);
        }

        debug
            .field("span", &self.span)
            .field("parent", &self.parent)
            .field("syntax_node_kind", &self.syntax_node_kind)
            .finish()
    }
}

/// Categorizes how a reference is used at its source site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// Relationship source endpoint reference.
    RelationshipSource,
    /// Relationship destination endpoint reference.
    RelationshipDestination,
    /// Instance target reference inside `containerInstance` / `softwareSystemInstance`.
    InstanceTarget,
    /// Deployment-layer relationship source endpoint reference.
    DeploymentRelationshipSource,
    /// Deployment-layer relationship destination endpoint reference.
    DeploymentRelationshipDestination,
    /// View scope reference for scoped views.
    ViewScope,
    /// `include` reference nested inside a view body.
    ViewInclude,
    /// `exclude` reference nested inside a view body.
    ViewExclude,
    /// Identifier-valued `animation` reference nested inside a view body.
    ViewAnimation,
}

/// Narrows which symbol kinds are valid targets for a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceTargetHint {
    /// The reference should resolve to an element symbol.
    Element,
    /// The reference should resolve to a deployment-layer symbol.
    Deployment,
    /// The reference should resolve to a relationship symbol.
    Relationship,
    /// The reference may resolve to either an element or relationship symbol.
    ElementOrRelationship,
}

/// Describes one raw symbol reference extracted from a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The syntactic role the reference plays at its source site.
    pub kind: ReferenceKind,
    /// Original identifier text for the reference.
    pub raw_text: String,
    /// Span of the referenced identifier token.
    pub span: TextSpan,
    /// Expected category of symbol the reference should target.
    pub target_hint: ReferenceTargetHint,
    /// Exact Tree-sitter node kind that contains the reference.
    pub container_node_kind: String,
    /// Nearest extracted symbol that lexically contains this reference.
    pub containing_symbol: Option<SymbolId>,
}

/// Supported `!identifiers` directive modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierMode {
    /// Flat identifiers such as `system`.
    Flat,
    /// Hierarchical identifiers such as `model.system`.
    Hierarchical,
    /// Any unrecognized raw mode string.
    Other(String),
}

impl IdentifierMode {
    // TODO: Could this be replaced with a From/Into trait implementation instead?
    pub(crate) fn from_raw(raw_value: &str) -> Self {
        match raw_value.to_ascii_lowercase().as_str() {
            "flat" => Self::Flat,
            "hierarchical" => Self::Hierarchical,
            _ => Self::Other(raw_value.to_owned()),
        }
    }
}

/// Captures one `!identifiers` directive and its parsed mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierModeFact {
    /// Normalized mode classification for the directive value.
    pub mode: IdentifierMode,
    /// Original directive value text, including any surrounding quotes.
    pub raw_value: String,
    /// Concrete syntax form used for the directive value.
    pub value_kind: DirectiveValueKind,
    /// Span of the full `!identifiers` directive.
    pub span: TextSpan,
    /// Span of just the directive value node.
    pub value_span: TextSpan,
    /// Nearest supported enclosing block for the directive.
    pub container: DirectiveContainer,
}
