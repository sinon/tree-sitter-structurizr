//! Higher-level syntax-backed facts used by later semantic validators.

use crate::includes::{DirectiveValueKind, normalized_directive_value};
use crate::span::TextSpan;

/// One extracted syntax value plus its normalized text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueFact {
    /// Original source text for the value, including any surrounding quotes.
    pub raw_text: String,
    /// Normalized text with string delimiters removed when applicable.
    pub normalized_text: String,
    /// Concrete syntax form that produced this value.
    pub value_kind: DirectiveValueKind,
    /// Source span covered by the value node.
    pub span: TextSpan,
}

impl ValueFact {
    pub(crate) fn new(raw_text: String, value_kind: DirectiveValueKind, span: TextSpan) -> Self {
        let normalized_text = normalized_directive_value(&raw_text, &value_kind);
        Self {
            raw_text,
            normalized_text,
            value_kind,
            span,
        }
    }
}

/// Top-level workspace section kinds that later validation cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSectionKind {
    /// A `model { ... }` section.
    Model,
    /// A `views { ... }` section.
    Views,
    /// A `configuration { ... }` section.
    Configuration,
}

/// One top-level section occurrence in the current document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSectionFact {
    /// The kind of top-level section that appeared.
    pub kind: WorkspaceSectionKind,
    /// Source span covered by the section node.
    pub span: TextSpan,
}

/// Normalized workspace-scope values extracted from configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceScope {
    /// The workspace is landscape-scoped.
    Landscape,
    /// The workspace is software-system-scoped.
    SoftwareSystem,
    /// The workspace is container-scoped.
    Container,
    /// The workspace is component-scoped.
    Component,
    /// Any other raw scope value.
    Other(String),
}

impl WorkspaceScope {
    pub(crate) fn from_raw(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "landscape" => Self::Landscape,
            "softwaresystem" => Self::SoftwareSystem,
            "container" => Self::Container,
            "component" => Self::Component,
            _ => Self::Other(raw.to_owned()),
        }
    }
}

/// One `configuration { scope ... }` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationScopeFact {
    /// Normalized scope classification.
    pub scope: WorkspaceScope,
    /// Original scope value and its source span.
    pub value: ValueFact,
    /// Source span covered by the whole `scope` statement.
    pub span: TextSpan,
}

/// One `properties { ... }` entry plus its owning block kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyFact {
    /// Extracted property name.
    pub name: ValueFact,
    /// Extracted property value.
    pub value: ValueFact,
    /// Source span covered by the whole property entry.
    pub span: TextSpan,
    /// Nearest enclosing owner node kind, such as `views_block`.
    pub container_node_kind: String,
}

/// File-backed directive kinds that later resource validation cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceDirectiveKind {
    /// A `!docs` directive.
    Docs,
    /// A `!adrs` directive.
    Adrs,
}

/// One `!docs` or `!adrs` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDirectiveFact {
    /// The directive family.
    pub kind: ResourceDirectiveKind,
    /// Extracted filesystem-like path value.
    pub path: ValueFact,
    /// Optional importer value attached to the directive.
    pub importer: Option<ValueFact>,
    /// Source span covered by the whole directive.
    pub span: TextSpan,
    /// Nearest enclosing owner node kind, such as `workspace_block`.
    pub container_node_kind: String,
}

/// One `!element` directive target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementDirectiveFact {
    /// Extracted selector or identifier target.
    pub target: ValueFact,
    /// Source span covered by the whole directive.
    pub span: TextSpan,
    /// Nearest enclosing owner node kind, such as `model_block`.
    pub container_node_kind: String,
}

/// Supported view-definition families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    /// A `systemLandscape` view.
    SystemLandscape,
    /// A `systemContext` view.
    SystemContext,
    /// A `container` view.
    Container,
    /// A `component` view.
    Component,
    /// A `filtered` view.
    Filtered,
    /// A `dynamic` view.
    Dynamic,
    /// A `deployment` view.
    Deployment,
    /// A `custom` view.
    Custom,
    /// An `image` view.
    Image,
}

/// One `autoLayout` statement captured from a view body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoLayoutFact {
    /// Source span covered by the whole `autoLayout` statement.
    pub span: TextSpan,
    /// Optional explicit layout direction.
    pub direction: Option<String>,
    /// Optional explicit rank separation.
    pub rank_separation: Option<String>,
    /// Optional explicit node separation.
    pub node_separation: Option<String>,
}

/// One declared model or deployment relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipFact {
    /// Source span covered by the whole relationship declaration.
    pub span: TextSpan,
    /// Source endpoint identifier.
    pub source: ValueFact,
    /// Destination endpoint identifier.
    pub destination: ValueFact,
    /// Optional description text.
    pub description: Option<ValueFact>,
    /// Optional technology text.
    pub technology: Option<ValueFact>,
}

/// One explicit dynamic-relationship step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRelationshipFact {
    /// Source span covered by the whole dynamic step.
    pub span: TextSpan,
    /// Optional explicit ordering prefix.
    pub order: Option<String>,
    /// Source endpoint identifier.
    pub source: ValueFact,
    /// Destination endpoint identifier.
    pub destination: ValueFact,
    /// Optional description text.
    pub description: Option<ValueFact>,
    /// Optional technology text.
    pub technology: Option<ValueFact>,
}

/// One dynamic step that references a named relationship binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicRelationshipReferenceFact {
    /// Source span covered by the whole dynamic step.
    pub span: TextSpan,
    /// Optional explicit ordering prefix.
    pub order: Option<String>,
    /// Referenced relationship identifier.
    pub relationship: ValueFact,
    /// Description text attached to the step.
    pub description: ValueFact,
}

/// One dynamic-view step, regardless of whether it spells endpoints or a binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicViewStepFact {
    /// A step with explicit source and destination identifiers.
    Relationship(Box<DynamicRelationshipFact>),
    /// A step that references a named relationship binding.
    RelationshipReference(Box<DynamicRelationshipReferenceFact>),
}

/// Image-source statement families supported inside `image` views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSourceKind {
    /// A `plantuml` source.
    PlantUml,
    /// A `mermaid` source.
    Mermaid,
    /// A `kroki` source.
    Kroki,
    /// A literal `image` source.
    Image,
}

/// Whether an image source belongs to the default, light, or dark set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSourceMode {
    /// A source declared directly in the image view body.
    Default,
    /// A source nested under `light { ... }`.
    Light,
    /// A source nested under `dark { ... }`.
    Dark,
}

/// One extracted source statement from an `image` view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSourceFact {
    /// The concrete source family.
    pub kind: ImageSourceKind,
    /// Which image-source mode owns this statement.
    pub mode: ImageSourceMode,
    /// Optional extra format argument used by `kroki`.
    pub format: Option<ValueFact>,
    /// Primary source value, such as a path or inline diagram text.
    pub value: ValueFact,
    /// Source span covered by the whole source statement.
    pub span: TextSpan,
}

/// One extracted view definition plus the body facts later rules need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewFact {
    /// The concrete view family.
    pub kind: ViewKind,
    /// Source span covered by the whole view declaration.
    pub span: TextSpan,
    /// Optional span of the view body block.
    pub body_span: Option<TextSpan>,
    /// Optional declared view key.
    pub key: Option<ValueFact>,
    /// Optional scope value for scoped views.
    pub scope: Option<ValueFact>,
    /// Optional deployment-environment value for deployment views.
    pub environment: Option<ValueFact>,
    /// Optional base-view key for filtered views.
    pub base_key: Option<ValueFact>,
    /// Optional filter mode for filtered views.
    pub filter_mode: Option<String>,
    /// Optional tag filter value for filtered views.
    pub filter_tags: Option<ValueFact>,
    /// Optional `autoLayout` statement captured from the body.
    pub auto_layout: Option<AutoLayoutFact>,
    /// Identifier-like values collected from `include` statements.
    pub include_values: Vec<ValueFact>,
    /// Identifier-like values collected from `exclude` statements.
    pub exclude_values: Vec<ValueFact>,
    /// Identifier-like values collected from `animation` blocks.
    pub animation_values: Vec<ValueFact>,
    /// Dynamic-view steps collected from the body.
    pub dynamic_steps: Vec<DynamicViewStepFact>,
    /// Image-source statements collected from image-view bodies.
    pub image_sources: Vec<ImageSourceFact>,
}
