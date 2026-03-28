//! References handler for bounded same-document and cross-file navigation.

use tower_lsp_server::ls_types::{Location, ReferenceParams};

use crate::{
    convert::positions::position_to_byte_offset,
    server::Backend,
};

/// Handles `textDocument/references` for the bounded navigation slice.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Missing data is
/// reported as `Ok(Some(Vec::new()))`.
pub async fn references(
    backend: &Backend,
    params: ReferenceParams,
) -> tower_lsp_server::jsonrpc::Result<Option<Vec<Location>>> {
    let locations = {
        let state = backend.state().read().await;
        let uri = &params.text_document_position.text_document.uri;
        let Some(document) = state.documents().get(uri) else {
            return Ok(Some(Vec::new()));
        };
        let Some(snapshot) = state.snapshot(uri) else {
            return Ok(Some(Vec::new()));
        };
        let Some(offset) = position_to_byte_offset(
            document.line_index(),
            params.text_document_position.position,
        ) else {
            return Ok(Some(Vec::new()));
        };
        let locations = super::navigation::reference_locations(
            &state,
            document,
            snapshot,
            offset,
            params.context.include_declaration,
        );
        drop(state);
        locations
    };

    Ok(Some(locations))
}
