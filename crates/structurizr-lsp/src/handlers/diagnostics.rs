//! Diagnostic helpers that keep syntax publishing logic out of text-sync code.

use structurizr_analysis::DocumentSnapshot;
use tower_lsp_server::ls_types::Diagnostic;

use crate::{convert, documents::DocumentState};

/// Converts one analyzed document snapshot into publishable LSP diagnostics.
#[must_use]
pub fn syntax_diagnostics(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
) -> Vec<Diagnostic> {
    convert::diagnostics::syntax_diagnostics(document, snapshot)
}
