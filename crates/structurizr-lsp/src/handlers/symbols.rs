//! Document symbol handler backed by extracted analysis symbols.

use tower_lsp_server::ls_types::{DocumentSymbolParams, DocumentSymbolResponse};

use crate::{convert, server::Backend};

pub async fn document_symbol(
    backend: &Backend,
    params: DocumentSymbolParams,
) -> tower_lsp_server::jsonrpc::Result<Option<DocumentSymbolResponse>> {
    let state = backend.state().read().await;
    let Some(document) = state.documents().get(&params.text_document.uri) else {
        return Ok(None);
    };
    let Some(snapshot) = state.snapshot(&params.text_document.uri) else {
        return Ok(None);
    };

    Ok(Some(DocumentSymbolResponse::Nested(
        convert::symbols::document_symbols(document, snapshot),
    )))
}
