//! Convert analysis diagnostics into LSP diagnostics.

use std::fs;

use structurizr_analysis::{
    DocumentId, DocumentSnapshot, IncludeDiagnostic, IncludeDiagnosticKind, SyntaxDiagnostic,
    WorkspaceFacts,
};
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};

use crate::{convert::positions::span_to_range, documents::DocumentState};

/// Converts syntax and include diagnostics into publishable LSP diagnostics.
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
    }

    diagnostics
}

fn syntax_diagnostic(
    document: &DocumentState,
    diagnostic: &SyntaxDiagnostic,
) -> Option<Diagnostic> {
    Some(Diagnostic {
        range: span_to_range(document.line_index(), diagnostic.span)?,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("structurizr-lsp".to_owned()),
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
        source: Some("structurizr-lsp".to_owned()),
        message: diagnostic.message.clone(),
        ..Diagnostic::default()
    })
}

fn workspace_document_id(document: &DocumentState) -> Option<DocumentId> {
    let path = document.uri().to_file_path()?;
    let canonical_path = fs::canonicalize(&path).ok()?;
    Some(DocumentId::new(
        canonical_path.to_string_lossy().into_owned(),
    ))
}
