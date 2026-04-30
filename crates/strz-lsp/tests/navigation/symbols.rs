use serde_json::json;

use crate::support::{
    file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders, initialized,
    new_service, open_document, request_json, workspace_fixture_path,
};

use super::shared::DIRECT_REFERENCES_SOURCE;

#[tokio::test(flavor = "current_thread")]
async fn document_symbols_follow_analysis_symbols() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, DIRECT_REFERENCES_SOURCE).await;

    let response = request_json(
        &mut service,
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri.as_str() }
        }),
    )
    .await;

    let symbols = response["result"]
        .as_array()
        .expect("document symbols should be returned as an array");
    let names = symbols
        .iter()
        .map(|symbol| {
            symbol["name"]
                .as_str()
                .expect("symbol name should be a string")
        })
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["User", "System", "Uses"]);
}

#[tokio::test(flavor = "current_thread")]
async fn workspace_symbols_load_workspace_before_documents_open() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("cross-file-navigation");
    let workspace_uri = file_uri_from_path(&workspace_root);

    initialize_with_workspace_folders(&mut service, &[workspace_uri]).await;
    initialized(&mut service).await;

    let response = request_json(
        &mut service,
        "workspace/symbol",
        json!({
            "query": "system"
        }),
    )
    .await;

    let symbols = workspace_symbols(&response);
    assert_eq!(symbol_names(symbols), vec!["System"]);
    assert_eq!(
        symbols[0]["location"]["uri"],
        file_uri_from_path(&workspace_root.join("model.dsl")).as_str()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn workspace_symbols_match_hierarchical_keys() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("hierarchical-identifiers");
    let workspace_uri = file_uri_from_path(&workspace_root);

    initialize_with_workspace_folders(&mut service, &[workspace_uri]).await;
    initialized(&mut service).await;

    let response = request_json(
        &mut service,
        "workspace/symbol",
        json!({
            "query": "system.api"
        }),
    )
    .await;

    let symbols = workspace_symbols(&response);
    assert_eq!(symbol_names(symbols), vec!["API"]);
    assert!(
        symbols[0]["containerName"]
            .as_str()
            .is_some_and(|container| container.contains("system.api")),
        "workspace symbol container should expose the canonical key"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn workspace_symbols_include_duplicate_bindings() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("duplicate-bindings");
    let workspace_uri = file_uri_from_path(&workspace_root);

    initialize_with_workspace_folders(&mut service, &[workspace_uri]).await;
    initialized(&mut service).await;

    let response = request_json(
        &mut service,
        "workspace/symbol",
        json!({
            "query": "api"
        }),
    )
    .await;

    let symbols = workspace_symbols(&response);
    assert_eq!(symbol_names(symbols), vec!["Alpha API", "Beta API"]);
}

#[tokio::test(flavor = "current_thread")]
async fn workspace_symbols_keep_shared_fragments_per_instance() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("multi-instance-open-fragment");
    let workspace_uri = file_uri_from_path(&workspace_root);

    initialize_with_workspace_folders(&mut service, &[workspace_uri]).await;
    initialized(&mut service).await;

    let response = request_json(
        &mut service,
        "workspace/symbol",
        json!({
            "query": "api"
        }),
    )
    .await;

    let symbols = workspace_symbols(&response);
    assert_eq!(symbol_names(symbols), vec!["API", "API"]);

    let containers = symbols
        .iter()
        .map(|symbol| {
            symbol["containerName"]
                .as_str()
                .expect("workspace symbols should include container names")
        })
        .collect::<Vec<_>>();
    assert!(
        containers
            .iter()
            .any(|container| container.contains("alpha.dsl")),
        "shared fragment symbol should be projected in the alpha workspace instance"
    );
    assert!(
        containers
            .iter()
            .any(|container| container.contains("beta.dsl")),
        "shared fragment symbol should be projected in the beta workspace instance"
    );
}

fn workspace_symbols(response: &serde_json::Value) -> &[serde_json::Value] {
    response["result"]
        .as_array()
        .expect("workspace symbols should be returned as an array")
}

fn symbol_names(symbols: &[serde_json::Value]) -> Vec<&str> {
    symbols
        .iter()
        .map(|symbol| {
            symbol["name"]
                .as_str()
                .expect("symbol name should be a string")
        })
        .collect()
}
