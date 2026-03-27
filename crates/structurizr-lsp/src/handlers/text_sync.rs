//! Full-document sync handlers that keep the latest snapshot in server state.

use std::{fs, path::PathBuf};

use structurizr_analysis::analyze_document;
use tower_lsp_server::ls_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use structurizr_analysis::{WorkspaceFacts, WorkspaceLoader};

use crate::{documents::DocumentState, handlers::diagnostics, server::Backend};

/// Handles `textDocument/didOpen` by analyzing and publishing the initial snapshot.
pub async fn did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
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
        return;
    };

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
    {
        let mut state = backend.state().write().await;
        state.documents_mut().close(&params.text_document.uri);
        state.remove_snapshot(&params.text_document.uri);
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

    publish_open_document_diagnostics(backend).await;
}

async fn publish_latest_snapshot(backend: &Backend, document: DocumentState) {
    let uri = document.uri().clone();
    let snapshot = analyze_document(document.to_input());
    let workspace_facts = recompute_workspace_facts(backend, Some(&document)).await;

    {
        let mut state = backend.state().write().await;
        state.documents_mut().open(document);
        state.set_snapshot(uri.clone(), snapshot);
        state.set_workspace_facts(workspace_facts);
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

    let load_paths = if workspace_roots.is_empty() {
        current_document
            .and_then(canonical_document_path)
            .map(|path| vec![path])
            .or_else(|| open_documents.iter().find_map(canonical_document_path).map(|path| vec![path]))
    } else {
        Some(workspace_roots)
    }?;

    let mut loader = WorkspaceLoader::new();

    for open_document in &open_documents {
        add_document_override(&mut loader, open_document);
    }

    if let Some(current_document) = current_document {
        add_document_override(&mut loader, current_document);
    }

    loader.load_paths(load_paths).ok()
}

fn add_document_override(loader: &mut WorkspaceLoader, document: &DocumentState) {
    if let Some(path) = canonical_document_path(document) {
        loader.set_document_override(path, document.text().to_owned());
    }
}

fn canonical_document_path(document: &DocumentState) -> Option<PathBuf> {
    canonical_file_path_from_uri(document.uri())
}

fn canonical_file_path_from_uri(uri: &tower_lsp_server::ls_types::Uri) -> Option<PathBuf> {
    let path = uri.to_file_path()?;
    fs::canonicalize(&path).ok()
}
