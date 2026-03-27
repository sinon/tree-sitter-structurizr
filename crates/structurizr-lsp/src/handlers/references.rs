//! Same-document references handler for the bounded first navigation slice.

use tower_lsp_server::ls_types::{Location, ReferenceParams};

use crate::{
    convert::positions::{position_to_byte_offset, span_to_range},
    server::Backend,
};

pub async fn references(
    backend: &Backend,
    params: ReferenceParams,
) -> tower_lsp_server::jsonrpc::Result<Option<Vec<Location>>> {
    let state = backend.state().read().await;
    let uri = &params.text_document_position.text_document.uri;
    let Some(document) = state.documents().get(uri) else {
        return Ok(Some(Vec::new()));
    };
    let Some(snapshot) = state.snapshot(uri) else {
        return Ok(Some(Vec::new()));
    };
    let Some(offset) =
        position_to_byte_offset(document.line_index(), params.text_document_position.position)
    else {
        return Ok(Some(Vec::new()));
    };
    let Some(symbol) = super::navigation::target_symbol_at_offset(snapshot, offset) else {
        return Ok(Some(Vec::new()));
    };

    let mut locations = Vec::new();

    if params.context.include_declaration {
        if let Some(range) = span_to_range(document.line_index(), symbol.span) {
            locations.push(Location::new(document.uri().clone(), range));
        }
    }

    locations.extend(
        super::navigation::references_for_symbol(snapshot, symbol)
            .into_iter()
            .filter_map(|reference| {
                span_to_range(document.line_index(), reference.span)
                    .map(|range| Location::new(document.uri().clone(), range))
            }),
    );

    Ok(Some(locations))
}
