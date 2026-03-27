mod support;

use serde_json::json;
use support::{
    file_uri, initialize, initialized, new_service, next_server_notification, open_document,
    position_in, request_json,
};

const DIRECT_REFERENCES_SOURCE: &str =
    include_str!("../../../tests/fixtures/lsp/relationships/named-relationships-ok.dsl");
const COMPLETION_SOURCE: &str = "workspace {\n  !i\n}\n";

#[tokio::test(flavor = "current_thread")]
async fn document_symbols_follow_analysis_symbols() {
    let (mut service, mut socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, DIRECT_REFERENCES_SOURCE).await;
    let _ = next_server_notification(&mut socket).await;

    let response = request_json(
        &mut service,
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri.as_str() }
        }),
        2,
    )
    .await;

    let symbols = response["result"]
        .as_array()
        .expect("document symbols should be returned as an array");
    let names = symbols
        .iter()
        .map(|symbol| symbol["name"].as_str().expect("symbol name should be a string"))
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["User", "System", "Uses"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_returns_directive_keywords_for_prefixes() {
    let (mut service, mut socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("completion.dsl");
    open_document(&mut service, &uri, COMPLETION_SOURCE).await;
    let _ = next_server_notification(&mut socket).await;

    let position = position_in(COMPLETION_SOURCE, "!i", 2);
    let response = request_json(
        &mut service,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
        3,
    )
    .await;

    let labels = response["result"]
        .as_array()
        .expect("completion should return an item array")
        .iter()
        .map(|item| item["label"].as_str().expect("completion label should be a string"))
        .collect::<Vec<_>>();

    assert!(labels.contains(&"!include"));
    assert!(labels.contains(&"!identifiers"));
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_same_document_relationship_references() {
    let (mut service, mut socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, DIRECT_REFERENCES_SOURCE).await;
    let _ = next_server_notification(&mut socket).await;

    let position = position_in(DIRECT_REFERENCES_SOURCE, "include rel", 8);
    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
        4,
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 5);
}

#[tokio::test(flavor = "current_thread")]
async fn references_include_definition_when_requested() {
    let (mut service, mut socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, DIRECT_REFERENCES_SOURCE).await;
    let _ = next_server_notification(&mut socket).await;

    let position = position_in(DIRECT_REFERENCES_SOURCE, "include rel", 8);
    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
            "context": { "includeDeclaration": true },
        }),
        5,
    )
    .await;

    let locations = response["result"]
        .as_array()
        .expect("references should return an array");
    let lines = locations
        .iter()
        .map(|location| {
            location["range"]["start"]["line"]
                .as_u64()
                .expect("location line should be numeric")
        })
        .collect::<Vec<_>>();

    assert_eq!(lines, vec![5, 10]);
}
