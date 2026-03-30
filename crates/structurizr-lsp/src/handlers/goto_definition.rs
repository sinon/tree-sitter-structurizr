//! Definition handler for bounded same-document and cross-file navigation.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use tower_lsp_server::ls_types::{
    GotoDefinitionParams, GotoDefinitionResponse, Location, Position, Range,
};
use tracing::{debug, info};

use crate::{
    convert::{positions::position_to_byte_offset, uris::file_uri_from_path},
    server::Backend,
};

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
        if let Some(location) =
            super::navigation::definition_location(&state, document, snapshot, offset)
        {
            drop(state);
            location
        } else if let Some(response) =
            directive_path_definition(snapshot, state.workspace_facts(), offset)
        {
            drop(state);
            return Ok(Some(response));
        } else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "gotoDefinition returned no result"
            );
            return Ok(None);
        }
    };

    info!(
        uri = uri.as_str(),
        ?position,
        "gotoDefinition resolved a definition target"
    );
    Ok(Some(GotoDefinitionResponse::Scalar(location)))
}

fn directive_path_definition(
    snapshot: &structurizr_analysis::DocumentSnapshot,
    workspace_facts: Option<&structurizr_analysis::WorkspaceFacts>,
    offset: usize,
) -> Option<GotoDefinitionResponse> {
    // Zed does not currently surface `textDocument/documentLink`, so keep the
    // richer link surface for other editors but also answer `definition` on
    // directive path spans as a compatibility fallback for Cmd-click navigation.
    // Reference: https://github.com/zed-industries/zed/issues/33587
    let mut locations = super::directive_paths::resolved_directive_paths_at_offset(
        snapshot,
        workspace_facts,
        offset,
    )
    .into_iter()
    .flat_map(|path| definition_targets_for_path(path.path()))
    .collect::<BTreeSet<_>>()
    .into_iter()
    .filter_map(|path| location_for_path(&path))
    .collect::<Vec<_>>();

    match locations.len() {
        0 => None,
        1 => Some(GotoDefinitionResponse::Scalar(
            locations.pop().expect("single location should exist"),
        )),
        _ => Some(GotoDefinitionResponse::Array(locations)),
    }
}

fn location_for_path(path: &Path) -> Option<Location> {
    let uri = file_uri_from_path(path)?;
    let origin = Position::new(0, 0);
    Some(Location::new(uri, Range::new(origin, origin)))
}

fn definition_targets_for_path(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }

    // Upstream Structurizr importers enumerate direct children in the supplied
    // docs/ADRs directory rather than walking nested subfolders. Mirroring that
    // behavior keeps the fallback cheap and aligned with what the DSL runtime
    // actually imports.
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut files = BTreeSet::new();

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_file() {
            files.insert(entry.path());
        }
    }

    files.into_iter().collect()
}
