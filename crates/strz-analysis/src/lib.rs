#![warn(missing_docs)]
//! Transport-agnostic analysis primitives for Structurizr DSL documents.
//!
//! This crate sits between the Tree-sitter grammar and a future LSP crate.
//! It owns reusable parsing outputs, extracted facts, and snapshot-oriented APIs.

mod constants;
mod diagnostics;
mod extract;
pub(crate) mod includes;
mod parse;
mod rule;
mod rules;
mod semantic;
mod snapshot;
mod span;
mod symbols;
mod tag_surfaces;
mod workspace;

pub use constants::ConstantDefinition;
pub use diagnostics::{Annotation, Diagnostic, RuledDiagnostic, diagnostic_rule_registry};
pub use includes::{DirectiveContainer, DirectiveValueKind, IncludeDirective};
pub use parse::DocumentAnalyzer;
pub use rule::{DiagnosticSeverity, RuleId, RuleMetadata, RuleRegistry, RuleRegistryBuilder};
pub use semantic::{
    AutoLayoutFact, ConfigurationScopeFact, DynamicRelationshipFact,
    DynamicRelationshipReferenceFact, DynamicViewStepFact, ElementDirectiveFact, ImageSourceFact,
    ImageSourceKind, ImageSourceMode, PropertyFact, RelationshipFact, ResourceDirectiveFact,
    ResourceDirectiveKind, ValueFact, ViewFact, ViewKind, WorkspaceScope, WorkspaceSectionFact,
    WorkspaceSectionKind,
};
pub use snapshot::{
    DocumentId, DocumentInput, DocumentLocation, DocumentSnapshot, DocumentSyntaxFacts,
};
pub use span::{TextPoint, TextSpan};
pub use symbols::{
    IdentifierMode, IdentifierModeFact, Reference, ReferenceKind, ReferenceTargetHint, Symbol,
    SymbolId, SymbolKind,
};
pub use tag_surfaces::{TagSurface, tag_surface_for_node_kind};
pub use workspace::{
    ElementIdentifierMode, ReferenceHandle, ReferenceResolutionStatus, ResolvedInclude,
    SymbolHandle, WorkspaceDocument, WorkspaceDocumentKind, WorkspaceFacts, WorkspaceIncludeTarget,
    WorkspaceIndex, WorkspaceInstanceId, WorkspaceLoadError, WorkspaceLoadFailure,
    WorkspaceLoadFailureAnchor, WorkspaceLoadFailureKind, WorkspaceLoader,
};
