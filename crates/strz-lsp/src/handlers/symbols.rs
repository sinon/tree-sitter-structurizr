//! Document symbol handler backed by extracted analysis symbols.

use tower_lsp_server::ls_types::{DocumentSymbolParams, DocumentSymbolResponse};

use crate::{convert, server::Backend};

/// Handles `textDocument/documentSymbol` for one open Structurizr document.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Missing documents are
/// reported as `Ok(None)`.
pub async fn document_symbol(
    backend: &Backend,
    params: DocumentSymbolParams,
) -> tower_lsp_server::jsonrpc::Result<Option<DocumentSymbolResponse>> {
    let symbols = {
        let state = backend.state().read().await;
        let Some(document) = state.documents().get(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(&params.text_document.uri) else {
            return Ok(None);
        };

        let symbols = convert::symbols::document_symbols(document, snapshot);
        drop(state);
        symbols
    };

    Ok(Some(DocumentSymbolResponse::Nested(symbols)))
}
