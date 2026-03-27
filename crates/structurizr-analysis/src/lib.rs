#![warn(missing_docs)]
//! Transport-agnostic analysis primitives for Structurizr DSL documents.
//!
//! This crate sits between the Tree-sitter grammar and a future LSP crate.
//! It owns reusable parsing outputs, extracted facts, and snapshot-oriented APIs.

mod diagnostics;
mod extract;
mod includes;
mod parse;
mod snapshot;
mod span;
mod symbols;
mod workspace;

pub use diagnostics::{SyntaxDiagnostic, SyntaxDiagnosticKind};
pub use includes::{DirectiveContainer, DirectiveValueKind, IncludeDirective};
pub use parse::{analyze_document, DocumentAnalyzer};
pub use snapshot::{DocumentId, DocumentInput, DocumentLocation, DocumentSnapshot};
pub use span::{TextPoint, TextSpan};
pub use symbols::{
    IdentifierMode, IdentifierModeFact, Reference, ReferenceKind, ReferenceTargetHint, Symbol,
    SymbolId, SymbolKind,
};
pub use workspace::WorkspaceFacts;
