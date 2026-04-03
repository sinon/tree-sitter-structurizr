mod support;

use std::fs;

use serde_json::json;
use support::{
    file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders, initialized,
    new_service, open_document, position_in, request_json, workspace_fixture_path,
};

const SAME_DOCUMENT_SOURCE: &str = r#"workspace {
    model {
        system = softwareSystem "Payments Platform" {
            api = container "Payments API" "Processes payment requests" "Rust" "Internal, HTTP" {
                technology "Axum"
                tags "Internal, Edge"
                url "https://example.com/api"
            }
            worker = container "Settlement Worker" "Settles payment jobs" "Rust"
        }

        rel = api -> worker "Publishes jobs" "NATS" "Async, Messaging" {
            description "Delivers asynchronous jobs"
            tag "Observed"
            url "https://example.com/rel"
        }
    }
}
"#;

const API_HOVER: &str = "**Container** `api`\nPayments API\n\nProcesses payment requests\n\n**Technology:** Axum  \n**Tags:** Internal, HTTP, Edge  \n**URL:** <https://example.com/api>";
const RELATIONSHIP_HOVER: &str = "**Relationship** `rel`\nPublishes jobs\n\nDelivers asynchronous jobs\n\n**Technology:** NATS  \n**Tags:** Async, Messaging, Observed  \n**URL:** <https://example.com/rel>";

#[tokio::test(flavor = "current_thread")]
async fn hover_returns_markdown_for_same_document_declarations() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("hover-same-document.dsl");
    open_document(&mut service, &uri, SAME_DOCUMENT_SOURCE).await;

    let hover = request_hover(
        &mut service,
        &uri,
        position_in(SAME_DOCUMENT_SOURCE, "api = container", 1),
        50,
    )
    .await;

    assert_hover_markdown(&hover, API_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_returns_markdown_for_same_document_references() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("hover-same-document.dsl");
    open_document(&mut service, &uri, SAME_DOCUMENT_SOURCE).await;

    let api_hover = request_hover(
        &mut service,
        &uri,
        position_in(SAME_DOCUMENT_SOURCE, "rel = api -> worker", 7),
        51,
    )
    .await;
    assert_hover_markdown(&api_hover, API_HOVER);

    let relationship_hover = request_hover(
        &mut service,
        &uri,
        position_in(SAME_DOCUMENT_SOURCE, "rel = api -> worker", 1),
        52,
    )
    .await;
    assert_hover_markdown(&relationship_hover, RELATIONSHIP_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_resolves_cross_file_symbols_through_workspace_indexes() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("hover-metadata");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let views_path = workspace_root.join("views.dsl");
    let views_source = read_workspace_file(&views_path);
    let views_uri = file_uri_from_path(&views_path);
    open_document(&mut service, &views_uri, &views_source).await;

    let api_hover = request_hover(
        &mut service,
        &views_uri,
        position_in(&views_source, "include api", 8),
        53,
    )
    .await;
    assert_hover_markdown(&api_hover, API_HOVER);

    let relationship_hover = request_hover(
        &mut service,
        &views_uri,
        position_in(&views_source, "include rel", 8),
        54,
    )
    .await;
    assert_hover_markdown(&relationship_hover, RELATIONSHIP_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_returns_no_result_for_ambiguous_workspace_references() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("duplicate-bindings");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let hover = request_hover(
        &mut service,
        &workspace_uri,
        position_in(&workspace_source, "user -> api", 8),
        55,
    )
    .await;

    assert!(hover["result"].is_null());
}

async fn request_hover(
    service: &mut support::TestService,
    uri: &tower_lsp_server::ls_types::Uri,
    position: tower_lsp_server::ls_types::Position,
    id: i64,
) -> serde_json::Value {
    request_json(
        service,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
        id,
    )
    .await
}

fn assert_hover_markdown(response: &serde_json::Value, expected: &str) {
    assert_eq!(response["result"]["contents"]["kind"], "markdown");
    assert_eq!(
        response["result"]["contents"]["value"]
            .as_str()
            .expect("hover markdown should be returned as a string"),
        expected
    );
}

fn read_workspace_file(path: &std::path::Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "workspace fixture `{}` should be readable: {error}",
            path.display()
        )
    })
}
