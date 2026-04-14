#![warn(missing_docs)]
//! Thin, testable LSP server scaffolding for Structurizr DSL editor features.

pub mod capabilities;
pub mod convert;
pub mod documents;
pub mod handlers;
pub(crate) mod identifier;
pub mod server;
pub mod state;

use tower_lsp_server::{LspService, Server};
use tracing::info;

pub use server::Backend;

/// Runs the stdio-backed LSP server that editor integrations launch via
/// `strz server`.
pub async fn serve_stdio() {
    info!(transport = "stdio", "starting Structurizr LSP server");
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
    info!(transport = "stdio", "Structurizr LSP server stopped");
}
