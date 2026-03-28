//! Transport-agnostic diagnostics derived from parsing and workspace discovery.

use crate::snapshot::DocumentId;
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

/// Categorizes include-resolution problems discovered while loading a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncludeDiagnosticKind {
    /// The include target did not exist on disk.
    MissingLocalTarget,
    /// The include target escaped or otherwise violated local path policy.
    EscapesAllowedSubtree,
    /// The include target participates in an explicit include cycle.
    IncludeCycle,
    /// The include target is remote and therefore left unresolved in the MVP.
    UnsupportedRemoteTarget,
}

/// Describes one include-resolution problem attached to a directive site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDiagnostic {
    /// The including document that should surface this diagnostic.
    pub document: DocumentId,
    /// The include-resolution category.
    pub kind: IncludeDiagnosticKind,
    /// Human-readable summary of the include problem.
    pub message: String,
    /// Normalized include target text with surrounding quotes stripped.
    pub target_text: String,
    /// Span of the full include directive in the including document.
    pub span: TextSpan,
    /// Span of the directive value node in the including document.
    pub value_span: TextSpan,
}

impl IncludeDiagnostic {
    pub(crate) fn missing_local_target(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::MissingLocalTarget,
            message: format!("included path does not exist: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
        }
    }

    pub(crate) fn escapes_allowed_subtree(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::EscapesAllowedSubtree,
            message: format!("included path escapes the allowed subtree: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
        }
    }

    pub(crate) fn include_cycle(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::IncludeCycle,
            message: format!("include cycle detected while following: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
        }
    }

    pub(crate) fn unsupported_remote_target(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::UnsupportedRemoteTarget,
            message: format!("remote includes are not resolved in the MVP: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
        }
    }
}

/// Categorizes bounded semantic diagnostics derived from workspace indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticDiagnosticKind {
    /// More than one definition claimed the same canonical binding key.
    DuplicateBinding,
    /// A supported identifier reference resolved to no known target.
    UnresolvedReference,
    /// A supported identifier reference could not be resolved confidently.
    AmbiguousReference,
}

/// Describes one semantic problem attached to a definition or reference site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticDiagnostic {
    /// The document that should surface this diagnostic.
    pub document: DocumentId,
    /// The semantic-diagnostic category.
    pub kind: SemanticDiagnosticKind,
    /// Human-readable summary of the semantic problem.
    pub message: String,
    /// Span of the affected symbol or reference.
    pub span: TextSpan,
}

impl SemanticDiagnostic {
    pub(crate) fn duplicate_binding(
        document: &DocumentId,
        binding_kind: &str,
        key: &str,
        span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::DuplicateBinding,
            message: format!("duplicate {binding_kind} binding: {key}"),
            span,
        }
    }

    pub(crate) fn unresolved_reference(document: &DocumentId, raw_text: &str, span: TextSpan) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::UnresolvedReference,
            message: format!("unresolved identifier reference: {raw_text}"),
            span,
        }
    }

    pub(crate) fn ambiguous_reference(document: &DocumentId, raw_text: &str, span: TextSpan) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::AmbiguousReference,
            message: format!("ambiguous identifier reference: {raw_text}"),
            span,
        }
    }
}
