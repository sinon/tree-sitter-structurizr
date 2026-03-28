//! Type-definition handler for instance-to-model navigation.

use tower_lsp_server::ls_types::request::{GotoTypeDefinitionParams, GotoTypeDefinitionResponse};
use tracing::{debug, info};

use crate::{convert::positions::position_to_byte_offset, server::Backend};

/// Handles `textDocument/typeDefinition` for instance declarations and
/// deployment references that represent model elements.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Missing targets are
/// reported as `Ok(None)`.
pub async fn goto_type_definition(
    backend: &Backend,
    params: GotoTypeDefinitionParams,
) -> tower_lsp_server::jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
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
                "gotoTypeDefinition skipped because the document is not open"
            );
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "gotoTypeDefinition skipped because no snapshot is cached"
            );
            return Ok(None);
        };
        let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "gotoTypeDefinition skipped because the position was invalid"
            );
            return Ok(None);
        };
        debug!(
            uri = uri.as_str(),
            ?position,
            offset,
            "running gotoTypeDefinition"
        );
        let Some(location) =
            super::navigation::type_definition_location(&state, document, snapshot, offset)
        else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "gotoTypeDefinition returned no result"
            );
            return Ok(None);
        };
        drop(state);
        location
    };

    info!(
        uri = uri.as_str(),
        ?position,
        "gotoTypeDefinition resolved a type definition target"
    );
    Ok(Some(GotoTypeDefinitionResponse::Scalar(location)))
}
