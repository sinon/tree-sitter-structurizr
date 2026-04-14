//! Server capability advertisement kept separate from handler code.

use tower_lsp_server::ls_types::{
    CompletionOptions, DocumentLinkOptions, HoverProviderCapability, OneOf, RenameOptions,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TypeDefinitionProviderCapability, WorkDoneProgressOptions,
};

const NON_ALPHANUMERIC_COMPLETION_TRIGGER_CHARACTERS: &[char] = &['!', '_'];

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
            trigger_characters: Some(completion_trigger_characters()),
            ..CompletionOptions::default()
        }),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        ..ServerCapabilities::default()
    }
}

/// Returns the characters that should trigger automatic completion requests.
///
/// The current bounded completion surface includes:
/// - directives that start with `!`
/// - fixed-vocabulary and style-property items that start with ASCII letters
/// - identifier completions that may start with ASCII letters, digits, or `_`
///
/// ASCII digits stay in the trigger set because bindable identifiers may begin
/// with them as long as they are not all digits, and suffix typing like
/// `system2` should still retrigger completion as the user refines the prefix.
///
/// We intentionally do not advertise `.` or `-` as trigger characters.
/// Hierarchical identifier completions are still suppressed, so `.` is only a
/// continuation character in unsupported forms, and neither the local grammar
/// nor the upstream parser allows identifiers to start with `-`.
fn completion_trigger_characters() -> Vec<String> {
    ('a'..='z')
        .chain('A'..='Z')
        .chain('0'..='9')
        .chain(
            NON_ALPHANUMERIC_COMPLETION_TRIGGER_CHARACTERS
                .iter()
                .copied(),
        )
        .map(|character| character.to_string())
        .collect()
}
