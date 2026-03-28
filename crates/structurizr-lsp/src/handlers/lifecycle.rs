//! Lifecycle handlers for server startup and shutdown.

use tower_lsp_server::ls_types::{
    InitializeParams, InitializeResult, InitializedParams, ServerInfo, Uri,
};
use tracing::{debug, info};

use crate::{capabilities, server::Backend};

/// Handles the LSP initialize request and captures session-level client state.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors.
pub async fn initialize(
    backend: &Backend,
    params: InitializeParams,
) -> tower_lsp_server::jsonrpc::Result<InitializeResult> {
    #[allow(deprecated)]
    let workspace_roots = params.workspace_folders.map_or_else(
        || params.root_uri.into_iter().collect::<Vec<Uri>>(),
        |folders| folders.into_iter().map(|folder| folder.uri).collect(),
    );
    info!(
        workspace_root_count = workspace_roots.len(),
        "initializing Structurizr LSP session"
    );
    debug!(workspace_roots = ?workspace_roots, "captured workspace roots");

    let mut state = backend.state().write().await;
    state.set_client_capabilities(params.capabilities);
    state.set_workspace_roots(workspace_roots);
    drop(state);

    Ok(InitializeResult {
        capabilities: capabilities::server_capabilities(),
        server_info: Some(ServerInfo {
            name: "strz".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
        ..InitializeResult::default()
    })
}

/// Handles the post-initialize notification.
pub fn initialized(_backend: &Backend, _params: InitializedParams) {
    info!("language server initialized");
}

/// Handles the LSP shutdown request.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors.
pub fn shutdown(_backend: &Backend) -> tower_lsp_server::jsonrpc::Result<()> {
    info!("shutting down Structurizr LSP session");
    Ok(())
}
