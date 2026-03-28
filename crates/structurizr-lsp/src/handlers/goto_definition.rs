//! Definition handler for bounded same-document and cross-file navigation.

use tower_lsp_server::ls_types::{GotoDefinitionParams, GotoDefinitionResponse};

use crate::{convert::positions::position_to_byte_offset, server::Backend};

/// Handles `textDocument/definition` for the bounded navigation slice.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Missing targets are
/// reported as `Ok(None)`.
pub async fn goto_definition(
    backend: &Backend,
    params: GotoDefinitionParams,
) -> tower_lsp_server::jsonrpc::Result<Option<GotoDefinitionResponse>> {
    let location = {
        let state = backend.state().read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(document) = state.documents().get(uri) else {
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte_offset(
            document.line_index(),
            params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let Some(location) = super::navigation::definition_location(&state, document, snapshot, offset)
        else {
            return Ok(None);
        };
        drop(state);
        location
    };

    Ok(Some(GotoDefinitionResponse::Scalar(location)))
}
