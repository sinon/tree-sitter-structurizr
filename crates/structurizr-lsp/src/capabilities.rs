//! Server capability advertisement kept separate from handler code.

use tower_lsp_server::ls_types::{
    CompletionOptions, OneOf, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions,
};

/// Builds the server capabilities advertised during LSP initialization.
#[must_use]
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: None,
            },
        )),
        document_symbol_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec!["!".to_owned()]),
            ..CompletionOptions::default()
        }),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        ..ServerCapabilities::default()
    }
}
