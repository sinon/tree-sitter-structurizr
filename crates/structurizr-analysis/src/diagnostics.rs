//! Syntax-diagnostic facts extracted from Tree-sitter parse trees.

use crate::span::TextSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxDiagnosticKind {
    ErrorNode,
    MissingNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxDiagnostic {
    pub kind: SyntaxDiagnosticKind,
    pub message: String,
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
