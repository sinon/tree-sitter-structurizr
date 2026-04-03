mod support;

use std::fs;

use support::{
    file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders, initialized,
    new_service, next_server_notification, open_document, workspace_fixture_path,
};

const INVALID_SOURCE: &str =
    include_str!("fixtures/directives/identifiers-unexpected-tokens-err.dsl");

#[tokio::test(flavor = "current_thread")]
async fn initialize_advertises_bounded_capabilities() {
    let (mut service, _) = new_service();
    let response = initialize(&mut service).await;
    let capabilities = &response["result"]["capabilities"];

    assert_eq!(capabilities["documentSymbolProvider"], true);
    assert_eq!(capabilities["hoverProvider"], true);
    assert_eq!(capabilities["definitionProvider"], true);
    assert_eq!(capabilities["typeDefinitionProvider"], true);
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
    assert!(
        !diagnostics.is_empty(),
        "invalid source should publish syntax diagnostics"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn did_open_publishes_include_diagnostics_for_file_backed_workspaces() {
    let (mut service, mut socket) = new_service();
    let workspace_root = workspace_fixture_path("missing-include");
    let workspace_uri = file_uri_from_path(&workspace_root);
    let workspace_file = workspace_root.join("workspace.dsl");
    let uri = file_uri_from_path(&workspace_file);
    let source = fs::read_to_string(&workspace_file).expect("workspace fixture should be readable");

    initialize_with_workspace_folders(&mut service, &[workspace_uri]).await;
    initialized(&mut service).await;
    open_document(&mut service, &uri, &source).await;

    let notification = next_server_notification(&mut socket).await;
    let diagnostics = notification["params"]["diagnostics"]
        .as_array()
        .expect("publishDiagnostics should include a diagnostics array");
    let messages = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["message"].as_str())
        .collect::<Vec<_>>();

    assert_eq!(notification["method"], "textDocument/publishDiagnostics");
    assert_eq!(notification["params"]["uri"], uri.as_str());
    assert!(
        messages
            .iter()
            .any(|message| message.contains("included path does not exist")),
        "workspace include diagnostics should be published for file-backed documents"
    );
}
