//! Completion handler for the initial fixed-vocabulary LSP slice.

use tower_lsp_server::ls_types::{CompletionParams, CompletionResponse};

use crate::{convert, server::Backend};

pub async fn completion(
    backend: &Backend,
    params: CompletionParams,
) -> tower_lsp_server::jsonrpc::Result<Option<CompletionResponse>> {
    let state = backend.state().read().await;
    let Some(document) = state
        .documents()
        .get(&params.text_document_position.text_document.uri)
    else {
        return Ok(None);
    };

    Ok(Some(CompletionResponse::Array(
        convert::completion::completion_items(
            document,
            params.text_document_position.position,
        ),
    )))
}
