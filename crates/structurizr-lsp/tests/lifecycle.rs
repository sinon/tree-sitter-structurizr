mod support;

use support::{file_uri, initialize, initialized, new_service, next_server_notification, open_document};

const INVALID_SOURCE: &str =
    include_str!("../../../tests/fixtures/lsp/directives/identifiers-unexpected-tokens-err.dsl");

#[tokio::test(flavor = "current_thread")]
async fn initialize_advertises_bounded_capabilities() {
    let (mut service, _) = new_service();
    let response = initialize(&mut service).await;
    let capabilities = &response["result"]["capabilities"];

    assert_eq!(capabilities["documentSymbolProvider"], true);
    assert_eq!(capabilities["definitionProvider"], true);
    assert_eq!(capabilities["referencesProvider"], true);
    assert_eq!(capabilities["textDocumentSync"]["change"], 1);
    assert!(capabilities["completionProvider"].is_object());
}

#[tokio::test(flavor = "current_thread")]
async fn did_open_publishes_syntax_diagnostics() {
    let (mut service, mut socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("identifiers-unexpected-tokens-err.dsl");
    open_document(&mut service, &uri, INVALID_SOURCE).await;

    let notification = next_server_notification(&mut socket).await;
    let diagnostics = notification["params"]["diagnostics"]
        .as_array()
        .expect("publishDiagnostics should include a diagnostics array");

    assert_eq!(notification["method"], "textDocument/publishDiagnostics");
    assert_eq!(notification["params"]["uri"], uri.as_str());
    assert!(!diagnostics.is_empty(), "invalid source should publish syntax diagnostics");
}
