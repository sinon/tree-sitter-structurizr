//! Diagnostic helpers that keep publishable diagnostics out of text-sync code.

use structurizr_analysis::{DocumentSnapshot, WorkspaceFacts};
use tower_lsp_server::ls_types::Diagnostic;

use crate::{convert, documents::DocumentState};

/// Converts one analyzed document snapshot plus optional workspace facts into
/// publishable LSP diagnostics.
#[must_use]
pub fn document_diagnostics(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
) -> Vec<Diagnostic> {
    convert::diagnostics::document_diagnostics(document, snapshot, workspace_facts)
}
