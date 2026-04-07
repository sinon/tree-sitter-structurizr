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
/// relationship source identifier immediately before an assigned
/// `deploymentEnvironment` statement. Without that guard, the editor can show a
/// cascaded syntax error for the stray recovery `=` token while the user is still
/// typing the relationship and relying on completion to finish it.
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

/// Suppress the stray `=` error produced by partial relationship recovery before an
/// assigned `deploymentEnvironment` statement.
///
/// The upstream Structurizr parser is effectively line-oriented, but our Tree-sitter
/// grammar can still recover a bare relationship-source line into the identifier on
/// the next assigned deployment-environment header. During an in-progress edit like:
///
/// ```text
/// customer
/// env = deploymentEnvironment "Prod" {
/// ```
///
/// recovery leaves a standalone `=` error on the deployment line even though the
/// rest of the line still forms a valid assigned `deploymentEnvironment` statement.
/// We keep this workaround in the LSP conversion layer and only suppress that exact
/// `=` artifact so genuine deployment-environment syntax errors still surface.
fn suppress_partial_relationship_recovery_diagnostic(
    document: &DocumentState,
    diagnostic: &SyntaxDiagnostic,
) -> bool {
    if diagnostic.kind != SyntaxDiagnosticKind::ErrorNode {
        return false;
    }

    if diagnostic.span.start_point.row != diagnostic.span.end_point.row {
        return false;
    }

    let Some(diagnostic_text) = document
        .text()
        .get(diagnostic.span.start_byte..diagnostic.span.end_byte)
        .map(str::trim)
    else {
        return false;
    };
    if diagnostic_text != "=" {
        return false;
    }

    let lines = document.text().split('\n').collect::<Vec<_>>();
    let current_line = lines
        .get(diagnostic.span.start_point.row)
        .map(|line| line.trim_end_matches('\r'));
    let previous_nonempty_line = lines
        .iter()
        .take(diagnostic.span.start_point.row)
        .rev()
        .map(|line| line.trim_end_matches('\r'))
        .find(|line| !line.trim().is_empty());

    matches!(
        (current_line, previous_nonempty_line),
        (Some(current_line), Some(previous_nonempty_line))
            if is_complete_assigned_deployment_environment_statement(current_line)
                && is_bare_identifier_line(previous_nonempty_line)
    )
}

fn is_complete_assigned_deployment_environment_statement(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = consume_identifier(trimmed) else {
        return false;
    };
    let Some(rest) = consume_prefix(consume_whitespace(rest), "=") else {
        return false;
    };
    let Some(rest) = consume_keyword(consume_whitespace(rest), "deploymentEnvironment") else {
        return false;
    };
    let Some(rest) = consume_value(consume_whitespace(rest)) else {
        return false;
    };

    matches!(consume_whitespace(rest), "" | "{")
}

fn consume_whitespace(line: &str) -> &str {
    line.trim_start_matches(char::is_whitespace)
}

fn consume_prefix<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.strip_prefix(prefix)
}

fn consume_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(keyword)?;
    rest.chars()
        .next()
        .is_none_or(char::is_whitespace)
        .then_some(rest)
}

fn consume_identifier(line: &str) -> Option<&str> {
    let mut end = 0;
    for (index, ch) in line.char_indices() {
        let is_valid = if index == 0 {
            ch.is_ascii_alphabetic() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-')
        };
        if !is_valid {
            break;
        }
        end = index + ch.len_utf8();
    }

    (end > 0).then_some(&line[end..])
}

fn consume_value(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix('"') {
        let mut escaped = false;
        for (index, ch) in rest.char_indices() {
            match (escaped, ch) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => return Some(&rest[index + ch.len_utf8()..]),
                (false, _) => {}
            }
        }
        None
    } else {
        consume_identifier(line)
    }
}

fn is_bare_identifier_line(line: &str) -> bool {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}
