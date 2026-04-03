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
mod snapshot;
mod span;
mod symbols;
mod workspace;

pub use constants::ConstantDefinition;
pub use diagnostics::{
    IncludeDiagnostic, IncludeDiagnosticKind, SemanticDiagnostic, SemanticDiagnosticKind,
    SyntaxDiagnostic, SyntaxDiagnosticKind,
};
pub use includes::{DirectiveContainer, DirectiveValueKind, IncludeDirective};
pub use parse::DocumentAnalyzer;
pub use snapshot::{
    DocumentId, DocumentInput, DocumentLocation, DocumentSnapshot, DocumentSyntaxFacts,
};
pub use span::{TextPoint, TextSpan};
pub use symbols::{
    IdentifierMode, IdentifierModeFact, Reference, ReferenceKind, ReferenceTargetHint, Symbol,
    SymbolId, SymbolKind,
};
pub use workspace::{
    ElementIdentifierMode, ReferenceHandle, ReferenceResolutionStatus, ResolvedInclude,
    SymbolHandle, WorkspaceDocument, WorkspaceDocumentKind, WorkspaceFacts, WorkspaceIncludeTarget,
    WorkspaceIndex, WorkspaceInstanceId, WorkspaceLoader,
};
