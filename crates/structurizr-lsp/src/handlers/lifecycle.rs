//! Lifecycle handlers for server startup and shutdown.

use tower_lsp_server::ls_types::{
    InitializeParams, InitializeResult, InitializedParams, ServerInfo, Uri,
};

use crate::{capabilities, server::Backend};

pub async fn initialize(
    backend: &Backend,
    params: InitializeParams,
) -> tower_lsp_server::jsonrpc::Result<InitializeResult> {
    #[allow(deprecated)]
    let workspace_roots = params
        .workspace_folders
        .map(|folders| folders.into_iter().map(|folder| folder.uri).collect())
        .unwrap_or_else(|| params.root_uri.into_iter().collect::<Vec<Uri>>());

    let mut state = backend.state().write().await;
    state.set_client_capabilities(params.capabilities);
    state.set_workspace_roots(workspace_roots);
    drop(state);

    Ok(InitializeResult {
        capabilities: capabilities::server_capabilities(),
        server_info: Some(ServerInfo {
            name: "structurizr-lsp".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
        ..InitializeResult::default()
    })
}

pub async fn initialized(_backend: &Backend, _params: InitializedParams) {}

pub async fn shutdown(_backend: &Backend) -> tower_lsp_server::jsonrpc::Result<()> {
    Ok(())
}
