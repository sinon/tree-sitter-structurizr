//! Same-document definition handler for the bounded first navigation slice.

use tower_lsp_server::ls_types::{GotoDefinitionParams, GotoDefinitionResponse, Location};

use crate::{
    convert::positions::{position_to_byte_offset, span_to_range},
    server::Backend,
};

/// Handles `textDocument/definition` within the current open document set.
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
        let Some(symbol) = super::navigation::target_symbol_at_offset(snapshot, offset) else {
            return Ok(None);
        };
        let Some(range) = span_to_range(document.line_index(), symbol.span) else {
            return Ok(None);
        };

        let location = Location::new(document.uri().clone(), range);
        drop(state);
        location
    };

    Ok(Some(GotoDefinitionResponse::Scalar(location)))
}
