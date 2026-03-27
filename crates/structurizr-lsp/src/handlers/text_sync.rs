//! Full-document sync handlers that keep the latest snapshot in server state.

use structurizr_analysis::analyze_document;
use tower_lsp_server::ls_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

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

    backend
        .client()
        .publish_diagnostics(params.text_document.uri, Vec::new(), None)
        .await;
}

async fn publish_latest_snapshot(backend: &Backend, document: DocumentState) {
    let uri = document.uri().clone();
    let version = document.version();
    let snapshot = analyze_document(document.to_input());
    let publishable_diagnostics = diagnostics::syntax_diagnostics(&document, &snapshot);

    {
        let mut state = backend.state().write().await;
        state.documents_mut().open(document);
        state.set_snapshot(uri.clone(), snapshot);
    }

    backend
        .client()
        .publish_diagnostics(uri, publishable_diagnostics, Some(version))
        .await;
}
