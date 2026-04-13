//! Convert analysis diagnostics into LSP diagnostics.

use line_index::LineIndex;
use strz_analysis::{
    Annotation, DiagnosticSeverity as AnalysisSeverity, DocumentId, DocumentSnapshot,
    RuledDiagnostic, WorkspaceFacts,
};
use tower_lsp_server::ls_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Uri,
};

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
        .filter_map(|diagnostic| analysis_diagnostic(document, diagnostic, workspace_facts, true))
        .collect::<Vec<_>>();

    if let Some(workspace_facts) = workspace_facts {
        diagnostics.extend(include_diagnostics(document, workspace_facts));
        diagnostics.extend(semantic_diagnostics(document, workspace_facts));
    }

    diagnostics
}

/// Convert one analysis diagnostic into an LSP diagnostic.
///
/// Syntax diagnostics pass through a narrow suppression step first because one
/// specific partial-edit state still recovers poorly in the grammar: a lone
/// relationship source identifier immediately before an assigned
/// `deploymentEnvironment` statement. Without that guard, the editor can show a
/// cascaded syntax error for the stray recovery `=` token while the user is still
/// typing the relationship and relying on completion to finish it.
fn analysis_diagnostic(
    document: &DocumentState,
    diagnostic: &RuledDiagnostic,
    workspace_facts: Option<&WorkspaceFacts>,
    allow_suppression: bool,
) -> Option<Diagnostic> {
    if allow_suppression && suppress_partial_relationship_recovery_diagnostic(document, diagnostic)
    {
        return None;
    }

    Some(Diagnostic {
        range: span_to_range(document.line_index(), diagnostic.span())?,
        severity: Some(to_lsp_severity(diagnostic.severity())),
        code: Some(NumberOrString::String(diagnostic.code().to_owned())),
        source: Some("strz".to_owned()),
        message: diagnostic.message().to_owned(),
        related_information: related_information(
            document,
            workspace_facts,
            diagnostic.annotations(),
        ),
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
        .filter_map(|diagnostic| {
            analysis_diagnostic(document, diagnostic, Some(workspace_facts), false)
        })
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
        .filter_map(|diagnostic| {
            analysis_diagnostic(document, diagnostic, Some(workspace_facts), false)
        })
        .collect()
}

fn workspace_document_id(document: &DocumentState) -> Option<DocumentId> {
    document.workspace_document_id().cloned()
}

const fn to_lsp_severity(severity: AnalysisSeverity) -> DiagnosticSeverity {
    match severity {
        AnalysisSeverity::Error => DiagnosticSeverity::ERROR,
        AnalysisSeverity::Warning => DiagnosticSeverity::WARNING,
    }
}

fn related_information(
    document: &DocumentState,
    workspace_facts: Option<&WorkspaceFacts>,
    annotations: &[Annotation],
) -> Option<Vec<DiagnosticRelatedInformation>> {
    let related = annotations
        .iter()
        .filter_map(|annotation| related_annotation(document, workspace_facts, annotation))
        .collect::<Vec<_>>();

    (!related.is_empty()).then_some(related)
}

fn related_annotation(
    document: &DocumentState,
    workspace_facts: Option<&WorkspaceFacts>,
    annotation: &Annotation,
) -> Option<DiagnosticRelatedInformation> {
    let location = annotation_location(document, workspace_facts, annotation)?;

    Some(DiagnosticRelatedInformation {
        location,
        message: annotation
            .message
            .clone()
            .unwrap_or_else(|| "related location".to_owned()),
    })
}

fn annotation_location(
    document: &DocumentState,
    workspace_facts: Option<&WorkspaceFacts>,
    annotation: &Annotation,
) -> Option<Location> {
    let Some(annotation_document) = annotation.document.as_ref() else {
        return Some(Location {
            uri: document.uri().clone(),
            range: span_to_range(document.line_index(), annotation.span)?,
        });
    };

    let workspace_facts = workspace_facts?;
    let related_document = workspace_facts.document(annotation_document)?;
    let snapshot = related_document.snapshot();
    let location = snapshot.location()?;
    let uri = Uri::from_file_path(location.path())?;
    let line_index = LineIndex::new(snapshot.source());

    Some(Location {
        uri,
        range: span_to_range(&line_index, annotation.span)?,
    })
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
    diagnostic: &RuledDiagnostic,
) -> bool {
    if diagnostic.code() != "syntax.error-node" {
        return false;
    }

    if diagnostic.span().start_point.row != diagnostic.span().end_point.row {
        return false;
    }

    let Some(diagnostic_text) = document
        .text()
        .get(diagnostic.span().start_byte..diagnostic.span().end_byte)
        .map(str::trim)
    else {
        return false;
    };
    if diagnostic_text != "=" {
        return false;
    }

    let lines = document.text().split('\n').collect::<Vec<_>>();
    let current_line = lines
        .get(diagnostic.span().start_point.row)
        .map(|line| line.trim_end_matches('\r'));
    let previous_nonempty_line = lines
        .iter()
        .take(diagnostic.span().start_point.row)
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
