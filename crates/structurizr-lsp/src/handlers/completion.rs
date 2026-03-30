//! Completion handler for the initial fixed-vocabulary LSP slice.

use structurizr_analysis::DocumentSnapshot;
use tower_lsp_server::ls_types::{CompletionParams, CompletionResponse};

use crate::{convert, server::Backend};

/// Handles `textDocument/completion` for the bounded fixed-vocabulary MVP.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Missing documents are
/// reported as `Ok(None)`.
pub async fn completion(
    backend: &Backend,
    params: CompletionParams,
) -> tower_lsp_server::jsonrpc::Result<Option<CompletionResponse>> {
    let items = {
        let state = backend.state().read().await;
        let uri = &params.text_document_position.text_document.uri;
        let Some(document) = state.documents().get(uri) else {
            return Ok(None);
        };
        let Some(snapshot): Option<&DocumentSnapshot> = state.snapshot(uri) else {
            return Ok(None);
        };

        let items = convert::completion::completion_items(
            document,
            snapshot,
            params.text_document_position.position,
        );
        drop(state);
        items
    };

    Ok(Some(CompletionResponse::Array(items)))
}
