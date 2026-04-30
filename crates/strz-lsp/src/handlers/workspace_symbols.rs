//! Workspace symbol handler backed by analysis-owned workspace facts.

use std::path::Path;

use line_index::LineIndex;
use strz_analysis::{
    DocumentId, DocumentLocation, Symbol, WorkspaceFacts, WorkspaceIndex, WorkspaceSymbolFact,
};
use tower_lsp_server::ls_types::{
    Location, SymbolInformation, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use tracing::{debug, info};

use crate::{
    convert::{positions::span_to_range, symbols::to_lsp_symbol_kind, uris::file_uri_from_path},
    documents::DocumentState,
    server::Backend,
    state::ServerState,
};

/// Handles `workspace/symbol` using cached or lazily loaded workspace facts.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Missing workspace facts
/// are reported as an empty symbol list.
#[allow(clippy::significant_drop_tightening)]
pub async fn workspace_symbol(
    backend: &Backend,
    params: WorkspaceSymbolParams,
) -> tower_lsp_server::jsonrpc::Result<Option<WorkspaceSymbolResponse>> {
    ensure_workspace_facts(backend).await;

    let symbols = {
        let state = backend.state().read().await;
        let Some(workspace_facts) = state.workspace_facts() else {
            debug!(
                query = params.query.as_str(),
                "workspace/symbol skipped because no workspace facts are available"
            );
            return Ok(Some(WorkspaceSymbolResponse::Flat(Vec::new())));
        };

        workspace_symbol_information(&state, workspace_facts, &params.query)
    };

    info!(
        query = params.query.as_str(),
        symbol_count = symbols.len(),
        "workspace/symbol completed"
    );
    Ok(Some(WorkspaceSymbolResponse::Flat(symbols)))
}

async fn ensure_workspace_facts(backend: &Backend) {
    {
        let state = backend.state().read().await;
        if state.workspace_facts().is_some() {
            return;
        }
    }

    let workspace_load = super::text_sync::recompute_workspace_facts(backend, None).await;
    let mut state = backend.state().write().await;
    if state.workspace_facts().is_none() {
        state.set_workspace_facts(workspace_load.facts);
        let _ = state.set_workspace_load_failures(workspace_load.failures);
    }
}

fn workspace_symbol_information(
    state: &ServerState,
    workspace_facts: &WorkspaceFacts,
    query: &str,
) -> Vec<SymbolInformation> {
    workspace_facts
        .workspace_indexes()
        .iter()
        .flat_map(WorkspaceIndex::workspace_symbols)
        .filter_map(|symbol_fact| symbol_information(state, workspace_facts, symbol_fact, query))
        .collect()
}

fn symbol_information(
    state: &ServerState,
    workspace_facts: &WorkspaceFacts,
    symbol_fact: &WorkspaceSymbolFact,
    query: &str,
) -> Option<SymbolInformation> {
    let snapshot = workspace_facts
        .document(symbol_fact.source_document())?
        .snapshot();
    let symbol = snapshot.symbols().get(symbol_fact.handle().symbol_id().0)?;
    if !matches_symbol_query(query, symbol_fact, symbol) {
        return None;
    }

    let open_document = open_document_by_id(state, symbol_fact.source_document());
    let location = symbol_location(
        open_document,
        snapshot.location(),
        snapshot.source(),
        symbol,
    )?;

    #[allow(deprecated)]
    Some(SymbolInformation {
        name: symbol.display_name.clone(),
        kind: to_lsp_symbol_kind(symbol.kind),
        tags: None,
        deprecated: None,
        location,
        container_name: Some(container_name(symbol_fact)),
    })
}

fn matches_symbol_query(query: &str, symbol_fact: &WorkspaceSymbolFact, symbol: &Symbol) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }

    let query = query.to_lowercase();
    field_matches(&symbol.display_name, &query)
        || symbol
            .binding_name
            .as_deref()
            .is_some_and(|binding_name| field_matches(binding_name, &query))
        || field_matches(symbol_fact.canonical_key(), &query)
}

fn field_matches(field: &str, query: &str) -> bool {
    field.to_lowercase().contains(query)
}

fn symbol_location(
    open_document: Option<&DocumentState>,
    location: Option<&DocumentLocation>,
    source: &str,
    symbol: &Symbol,
) -> Option<Location> {
    let span = symbol.binding_span.unwrap_or(symbol.span);

    if let Some(document) = open_document {
        let range = span_to_range(document.line_index(), span)?;
        return Some(Location::new(document.uri().clone(), range));
    }

    let line_index = LineIndex::new(source);
    let range = span_to_range(&line_index, span)?;
    let uri = file_uri_from_path(location?.path())?;
    Some(Location::new(uri, range))
}

fn container_name(symbol_fact: &WorkspaceSymbolFact) -> String {
    format!(
        "{} @ {}",
        symbol_fact.canonical_key(),
        document_label(symbol_fact.root_document())
    )
}

fn document_label(document_id: &DocumentId) -> &str {
    Path::new(document_id.as_str())
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_else(|| document_id.as_str())
}

fn open_document_by_id<'a>(
    state: &'a ServerState,
    document_id: &DocumentId,
) -> Option<&'a DocumentState> {
    state.documents().iter().find(|document| {
        document
            .workspace_document_id()
            .is_some_and(|candidate| candidate == document_id)
    })
}
