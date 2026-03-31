//! Full-document sync handlers that keep the latest snapshot in server state.

use std::{fs, path::PathBuf, time::Instant};

use structurizr_analysis::{DocumentId, analyze_document};
use tower_lsp_server::ls_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};
use tracing::{debug, info, warn};

use structurizr_analysis::{WorkspaceFacts, WorkspaceLoader};

use crate::{documents::DocumentState, handlers::diagnostics, server::Backend};

/// Handles `textDocument/didOpen` by analyzing and publishing the initial snapshot.
pub async fn did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
    info!(
        uri = params.text_document.uri.as_str(),
        version = params.text_document.version,
        text_bytes = params.text_document.text.len(),
        "handling didOpen"
    );
    let document = DocumentState::new(
        params.text_document.uri,
        params.text_document.version,
        params.text_document.text,
    );

    publish_latest_snapshot(backend, document).await;
}

/// Handles `textDocument/didChange` using full-document synchronization.
pub async fn did_change(backend: &Backend, params: DidChangeTextDocumentParams) {
    let Some(updated_text) = params
        .content_changes
        .into_iter()
        .last()
        .map(|change| change.text)
    else {
        debug!(
            uri = params.text_document.uri.as_str(),
            "ignoring didChange without a full-text payload"
        );
        return;
    };
    info!(
        uri = params.text_document.uri.as_str(),
        version = params.text_document.version,
        text_bytes = updated_text.len(),
        "handling didChange"
    );

    let updated_document = {
        let mut state = backend.state().write().await;
        if let Some(document) = state.documents_mut().get_mut(&params.text_document.uri) {
            document.replace_text(params.text_document.version, updated_text);
            document.clone()
        } else {
            let document = DocumentState::new(
                params.text_document.uri.clone(),
                params.text_document.version,
                updated_text,
            );
            state.documents_mut().open(document.clone());
            document
        }
    };

    publish_latest_snapshot(backend, updated_document).await;
}

/// Handles `textDocument/didClose` by clearing cached state and diagnostics.
pub async fn did_close(backend: &Backend, params: DidCloseTextDocumentParams) {
    info!(uri = params.text_document.uri.as_str(), "handling didClose");
    {
        let mut state = backend.state().write().await;
        state.documents_mut().close(&params.text_document.uri);
        state.remove_snapshot(&params.text_document.uri);
        debug!(
            open_document_count = state.documents().len(),
            "removed closed document from server state"
        );
    }

    let workspace_facts = recompute_workspace_facts(backend, None).await;

    {
        let mut state = backend.state().write().await;
        state.set_workspace_facts(workspace_facts);
    }

    backend
        .client()
        .publish_diagnostics(params.text_document.uri, Vec::new(), None)
        .await;
    info!(
        diagnostic_count = 0,
        clear_on_close = true,
        "published clear-diagnostics notification for closed document"
    );

    publish_open_document_diagnostics(backend).await;
}

async fn publish_latest_snapshot(backend: &Backend, document: DocumentState) {
    let uri = document.uri().clone();
    let workspace_facts = recompute_workspace_facts(backend, Some(&document)).await;
    // When workspace recomputation already analyzed this file-backed document
    // through `WorkspaceLoader`, reuse that snapshot instead of immediately
    // parsing and extracting the same document a second time.
    let snapshot = snapshot_from_workspace_facts(&document, workspace_facts.as_ref())
        .unwrap_or_else(|| analyze_document(document.to_input()));
    debug!(
        uri = uri.as_str(),
        syntax_diagnostic_count = snapshot.syntax_diagnostics().len(),
        symbol_count = snapshot.symbols().len(),
        reference_count = snapshot.references().len(),
        "analyzed latest document snapshot"
    );

    {
        let mut state = backend.state().write().await;
        state.documents_mut().open(document);
        state.set_snapshot(uri.clone(), snapshot);
        state.set_workspace_facts(workspace_facts);
        debug!(
            uri = uri.as_str(),
            open_document_count = state.documents().len(),
            "stored latest snapshot and workspace facts"
        );
    }

    publish_open_document_diagnostics(backend).await;
}

async fn publish_open_document_diagnostics(backend: &Backend) {
    let publish_jobs = {
        let state = backend.state().read().await;
        let workspace_facts = state.workspace_facts();

        state
            .documents()
            .iter()
            .filter_map(|document| {
                let snapshot = state.snapshot(document.uri())?;

                Some((
                    document.uri().clone(),
                    document.version(),
                    diagnostics::document_diagnostics(document, snapshot, workspace_facts),
                ))
            })
            .collect::<Vec<_>>()
    };

    for (uri, version, publishable_diagnostics) in publish_jobs {
        let diagnostic_count = publishable_diagnostics.len();
        info!(
            uri = uri.as_str(),
            version,
            diagnostic_count,
            clear_on_close = false,
            "publishing diagnostics"
        );
        backend
            .client()
            .publish_diagnostics(uri, publishable_diagnostics, Some(version))
            .await;
    }
}

async fn recompute_workspace_facts(
    backend: &Backend,
    current_document: Option<&DocumentState>,
) -> Option<WorkspaceFacts> {
    let (workspace_roots, open_documents) = {
        let state = backend.state().read().await;

        (
            state
                .workspace_roots()
                .iter()
                .filter_map(canonical_file_path_from_uri)
                .collect::<Vec<_>>(),
            state.documents().iter().cloned().collect::<Vec<_>>(),
        )
    };
    let current_uri = current_document.map(|document| document.uri().to_string());

    // Without configured workspace roots, anchor discovery from the current
    // document first and then any other open document so include diagnostics
    // still have a local workspace to resolve against.
    let load_paths = if workspace_roots.is_empty() {
        current_document
            .and_then(canonical_document_path)
            .map(|path| vec![path])
            .or_else(|| {
                open_documents
                    .iter()
                    .find_map(canonical_document_path)
                    .map(|path| vec![path])
            })
    } else {
        Some(workspace_roots)
    }?;
    let start = Instant::now();
    debug!(
        current_document_uri = current_uri.as_deref(),
        workspace_root_count = load_paths.len(),
        open_document_count = open_documents.len(),
        load_paths = ?load_paths,
        "recomputing workspace facts"
    );

    let mut loader = backend
        .workspace_loader()
        .lock()
        .expect("workspace loader mutex should not be poisoned");
    loader.clear_document_overrides();

    for open_document in &open_documents {
        add_document_override(&mut loader, open_document);
    }

    if let Some(current_document) = current_document {
        add_document_override(&mut loader, current_document);
    }

    match loader.load_paths(load_paths) {
        Ok(workspace_facts) => {
            info!(
                current_document_uri = current_uri.as_deref(),
                document_count = workspace_facts.documents().len(),
                include_count = workspace_facts.includes().len(),
                workspace_instance_count = workspace_facts.workspace_indexes().len(),
                semantic_diagnostic_count = workspace_facts.semantic_diagnostics().len(),
                elapsed_ms = start.elapsed().as_millis(),
                "recomputed workspace facts"
            );
            Some(workspace_facts)
        }
        Err(error) => {
            warn!(
                current_document_uri = current_uri.as_deref(),
                elapsed_ms = start.elapsed().as_millis(),
                error = %error,
                "failed to recompute workspace facts"
            );
            None
        }
    }
}

fn snapshot_from_workspace_facts(
    document: &DocumentState,
    workspace_facts: Option<&WorkspaceFacts>,
) -> Option<structurizr_analysis::DocumentSnapshot> {
    let document_id = workspace_document_id(document)?;
    workspace_facts
        .and_then(|facts| facts.document(&document_id))
        .map(|workspace_document| workspace_document.snapshot().clone())
}

fn workspace_document_id(document: &DocumentState) -> Option<DocumentId> {
    document.workspace_document_id().cloned()
}

fn add_document_override(loader: &mut WorkspaceLoader, document: &DocumentState) {
    if let Some(path) = canonical_document_path(document) {
        loader.set_document_override(path, document.text().to_owned());
    }
}

fn canonical_document_path(document: &DocumentState) -> Option<PathBuf> {
    document.canonical_path().cloned()
}

fn canonical_file_path_from_uri(uri: &tower_lsp_server::ls_types::Uri) -> Option<PathBuf> {
    let path = uri.to_file_path()?;
    fs::canonicalize(&path).ok()
}
