//! The `tower-lsp-server` backend stays thin and delegates to handler modules.

use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp_server::ls_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    GotoDefinitionResponse, InitializeParams, InitializeResult, InitializedParams, Location,
    ReferenceParams,
};
use tower_lsp_server::{Client, LanguageServer};

use crate::{handlers, state::ServerState};

/// Thin `tower-lsp-server` backend that delegates protocol work to handlers.
pub struct Backend {
    client: Client,
    state: Arc<RwLock<ServerState>>,
}

impl Backend {
    /// Creates a backend with an empty shared server state.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ServerState::default())),
        }
    }

    /// Returns the LSP client handle used for server-to-client notifications.
    #[must_use]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the shared mutable server state for protocol handlers.
    #[must_use]
    pub const fn state(&self) -> &Arc<RwLock<ServerState>> {
        &self.state
    }
}

impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp_server::jsonrpc::Result<InitializeResult> {
        handlers::lifecycle::initialize(self, params).await
    }

    async fn initialized(&self, params: InitializedParams) {
        handlers::lifecycle::initialized(self, params);
    }

    async fn shutdown(&self) -> tower_lsp_server::jsonrpc::Result<()> {
        handlers::lifecycle::shutdown(self)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        handlers::text_sync::did_open(self, params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        handlers::text_sync::did_change(self, params).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        handlers::text_sync::did_close(self, params).await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        handlers::symbols::document_symbol(self, params).await
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<CompletionResponse>> {
        handlers::completion::completion(self, params).await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        handlers::goto_definition::goto_definition(self, params).await
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<Vec<Location>>> {
        handlers::references::references(self, params).await
    }
}
