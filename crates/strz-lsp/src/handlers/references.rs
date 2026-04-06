//! References handler for bounded same-document and cross-file navigation.

use tower_lsp_server::ls_types::{Location, ReferenceParams};
use tracing::{debug, info};

use crate::{convert::positions::position_to_byte_offset, server::Backend};

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
    let uri = params.text_document_position.text_document.uri.clone();
    let position = params.text_document_position.position;
    let locations = {
        let state = backend.state().read().await;
        let Some(document) = state.documents().get(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "references skipped because the document is not open"
            );
            return Ok(Some(Vec::new()));
        };
        let Some(snapshot) = state.snapshot(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "references skipped because no snapshot is cached"
            );
            return Ok(Some(Vec::new()));
        };
        let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "references skipped because the position was invalid"
            );
            return Ok(Some(Vec::new()));
        };
        debug!(
            uri = uri.as_str(),
            ?position,
            offset,
            include_declaration = params.context.include_declaration,
            "running references"
        );
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

    info!(
        uri = uri.as_str(),
        ?position,
        location_count = locations.len(),
        "references completed"
    );
    Ok(Some(locations))
}
