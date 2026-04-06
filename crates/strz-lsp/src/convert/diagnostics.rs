//! Convert analysis diagnostics into LSP diagnostics.

use strz_analysis::{
    DocumentId, DocumentSnapshot, IncludeDiagnostic, IncludeDiagnosticKind, SemanticDiagnostic,
    SyntaxDiagnostic, SyntaxDiagnosticKind, WorkspaceFacts,
};
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};

use crate::{convert::positions::span_to_range, documents::DocumentState};

/// Converts syntax, include, and bounded semantic diagnostics into publishable LSP diagnostics.
#[must_use]
pub fn document_diagnostics(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
) -> Vec<Diagnostic> {
    let mut diagnostics = snapshot
        .syntax_diagnostics()
        .iter()
        .filter_map(|diagnostic| syntax_diagnostic(document, diagnostic))
        .collect::<Vec<_>>();

    if let Some(workspace_facts) = workspace_facts {
        diagnostics.extend(include_diagnostics(document, workspace_facts));
        diagnostics.extend(semantic_diagnostics(document, workspace_facts));
    }

    diagnostics
}

/// Convert one syntax diagnostic into an LSP diagnostic.
///
/// Every syntax diagnostic passes through a narrow suppression step first because one
/// specific partial-edit state still recovers poorly in the grammar: a lone
/// relationship source identifier immediately before a `deploymentEnvironment`
/// statement. Without that guard, the editor shows a cascaded syntax error on the
/// deployment line while the user is still typing the relationship and relying on
/// completion to finish it.
fn syntax_diagnostic(
    document: &DocumentState,
    diagnostic: &SyntaxDiagnostic,
) -> Option<Diagnostic> {
    if suppress_partial_relationship_recovery_diagnostic(document, diagnostic) {
        return None;
    }

    Some(Diagnostic {
        range: span_to_range(document.line_index(), diagnostic.span)?,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("strz".to_owned()),
        message: diagnostic.message.clone(),
        ..Diagnostic::default()
    })
}

fn include_diagnostics(
    document: &DocumentState,
    workspace_facts: &WorkspaceFacts,
) -> Vec<Diagnostic> {
    let Some(document_id) = workspace_document_id(document) else {
        return Vec::new();
    };

    workspace_facts
        .include_diagnostics_for(&document_id)
        .filter_map(|diagnostic| include_diagnostic(document, diagnostic))
        .collect()
}

fn semantic_diagnostics(
    document: &DocumentState,
    workspace_facts: &WorkspaceFacts,
) -> Vec<Diagnostic> {
    let Some(document_id) = workspace_document_id(document) else {
        return Vec::new();
    };

    workspace_facts
        .semantic_diagnostics_for(&document_id)
        .filter_map(|diagnostic| semantic_diagnostic(document, diagnostic))
        .collect()
}

fn include_diagnostic(
    document: &DocumentState,
    diagnostic: &IncludeDiagnostic,
) -> Option<Diagnostic> {
    Some(Diagnostic {
        range: span_to_range(document.line_index(), diagnostic.span)?,
        severity: Some(match diagnostic.kind {
            IncludeDiagnosticKind::UnsupportedRemoteTarget => DiagnosticSeverity::WARNING,
            IncludeDiagnosticKind::MissingLocalTarget
            | IncludeDiagnosticKind::EscapesAllowedSubtree
            | IncludeDiagnosticKind::IncludeCycle => DiagnosticSeverity::ERROR,
        }),
        source: Some("strz".to_owned()),
        message: diagnostic.message.clone(),
        ..Diagnostic::default()
    })
}

fn semantic_diagnostic(
    document: &DocumentState,
    diagnostic: &SemanticDiagnostic,
) -> Option<Diagnostic> {
    Some(Diagnostic {
        range: span_to_range(document.line_index(), diagnostic.span)?,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("strz".to_owned()),
        message: diagnostic.message.clone(),
        ..Diagnostic::default()
    })
}

fn workspace_document_id(document: &DocumentState) -> Option<DocumentId> {
    document.workspace_document_id().cloned()
}

/// Suppress the transient `deploymentEnvironment` error produced by partial
/// relationship recovery.
///
/// The upstream Structurizr parser is effectively line-oriented, but our Tree-sitter
/// grammar currently treats a bare identifier on the previous line as recoverable
/// input. During an in-progress edit like:
///
/// ```text
/// customer
/// deploymentEnvironment "Prod" {
/// ```
///
/// recovery can attach the resulting `ERROR` node to the `deploymentEnvironment`
/// statement instead of the incomplete relationship source. We keep this workaround in
/// the LSP conversion layer so it stays tightly scoped to that known editor-only
/// recovery artifact rather than weakening syntax diagnostics more broadly.
fn suppress_partial_relationship_recovery_diagnostic(
    document: &DocumentState,
    diagnostic: &SyntaxDiagnostic,
) -> bool {
    if diagnostic.kind != SyntaxDiagnosticKind::ErrorNode {
        return false;
    }

    let Some(range) = span_to_range(document.line_index(), diagnostic.span) else {
        return false;
    };
    let lines = document.text().split('\n').collect::<Vec<_>>();
    let current_line = lines
        .get(range.start.line as usize)
        .map(|line| line.trim_end_matches('\r'));
    let previous_nonempty_line = lines
        .iter()
        .take(range.start.line as usize)
        .rev()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| !line.trim().is_empty());

    matches!(
        (current_line, previous_nonempty_line),
        (Some(current_line), Some(previous_nonempty_line))
            if is_deployment_environment_statement(current_line)
                && is_bare_identifier_line(previous_nonempty_line)
    )
}

fn is_deployment_environment_statement(line: &str) -> bool {
    let trimmed = line.trim_start();
    starts_with_keyword(trimmed, "deploymentEnvironment")
        || trimmed.split_once('=').is_some_and(|(_, rest)| {
            starts_with_keyword(rest.trim_start(), "deploymentEnvironment")
        })
}

fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    line == keyword
        || line
            .strip_prefix(keyword)
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

fn is_bare_identifier_line(line: &str) -> bool {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}
