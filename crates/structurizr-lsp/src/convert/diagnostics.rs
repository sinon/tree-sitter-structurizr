//! Convert analysis diagnostics into LSP diagnostics.

use structurizr_analysis::{DocumentSnapshot, SyntaxDiagnostic};
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};

use crate::{convert::positions::span_to_range, documents::DocumentState};

/// Converts syntax diagnostics from one snapshot into publishable LSP diagnostics.
#[must_use]
pub fn syntax_diagnostics(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
) -> Vec<Diagnostic> {
    snapshot
        .syntax_diagnostics()
        .iter()
        .filter_map(|diagnostic| syntax_diagnostic(document, diagnostic))
        .collect()
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
