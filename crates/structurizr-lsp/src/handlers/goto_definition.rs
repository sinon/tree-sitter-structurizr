//! Definition handler for bounded same-document and cross-file navigation.

use tower_lsp_server::ls_types::{GotoDefinitionParams, GotoDefinitionResponse};
use tracing::{debug, info};

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
    let uri = params
        .text_document_position_params
        .text_document
        .uri
        .clone();
    let position = params.text_document_position_params.position;
    let location = {
        let state = backend.state().read().await;
        let Some(document) = state.documents().get(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "gotoDefinition skipped because the document is not open"
            );
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "gotoDefinition skipped because no snapshot is cached"
            );
            return Ok(None);
        };
        let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "gotoDefinition skipped because the position was invalid"
            );
            return Ok(None);
        };
        debug!(
            uri = uri.as_str(),
            ?position,
            offset,
            "running gotoDefinition"
        );
        let Some(location) =
            super::navigation::definition_location(&state, document, snapshot, offset)
        else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "gotoDefinition returned no result"
            );
            return Ok(None);
        };
        drop(state);
        location
    };

    info!(
        uri = uri.as_str(),
        ?position,
        "gotoDefinition resolved a definition target"
    );
    Ok(Some(GotoDefinitionResponse::Scalar(location)))
}
