use serde_json::json;

use crate::support::{file_uri, initialize, initialized, new_service, open_document, request_json};

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
