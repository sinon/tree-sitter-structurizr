//! Syntax-diagnostic facts extracted from Tree-sitter parse trees.

use crate::span::TextSpan;

/// Categorizes the syntax problem Tree-sitter reported for a source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxDiagnosticKind {
    /// Tree-sitter produced an `ERROR` node for unexpected syntax.
    ErrorNode,
    /// Tree-sitter synthesized a `MISSING` node to recover from absent syntax.
    MissingNode,
}

/// Describes a syntax problem extracted from the parse tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxDiagnostic {
    /// The parser-reported category of syntax problem.
    pub kind: SyntaxDiagnosticKind,
    /// Human-readable summary of the syntax problem.
    pub message: String,
    /// Byte and point range covered by the diagnostic.
    pub span: TextSpan,
}

impl SyntaxDiagnostic {
    pub(crate) fn unexpected_syntax(span: TextSpan) -> Self {
        Self {
            kind: SyntaxDiagnosticKind::ErrorNode,
            message: "unexpected syntax".to_owned(),
            span,
        }
    }

    pub(crate) fn missing_node(kind: &str, span: TextSpan) -> Self {
        Self {
            kind: SyntaxDiagnosticKind::MissingNode,
            message: format!("missing {kind}"),
            span,
        }
    }
}
