use std::fs;

use serde_json::json;

use crate::support::{
    file_uri_from_path, initialize_with_workspace_folders, initialized, new_service, open_document,
    read_workspace_file, request_json,
};

use super::shared::copied_workspace_fixture;

#[tokio::test(flavor = "current_thread")]
async fn document_links_resolve_docs_and_adrs_directive_paths() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let details_path = workspace_root.join("model/internet-banking-system/details.dsl");
    let details_source = read_workspace_file(&details_path);
    let details_uri = file_uri_from_path(&details_path);
    open_document(&mut service, &details_uri, &details_source).await;

    let response = request_json(
        &mut service,
        "textDocument/documentLink",
        json!({
            "textDocument": { "uri": details_uri.as_str() }
        }),
    )
    .await;

    let links = response["result"]
        .as_array()
        .expect("document links should return an item array");
    let docs_uri = file_uri_from_path(
        &details_path
            .parent()
            .expect("details path should have a parent")
            .join("docs"),
    );
    let adrs_uri = file_uri_from_path(
        &details_path
            .parent()
            .expect("details path should have a parent")
            .join("adrs"),
    );
    assert!(links.iter().any(|link| link["target"] == docs_uri.as_str()));
    assert!(links.iter().any(|link| link["target"] == adrs_uri.as_str()));

    let docs_link = links
        .iter()
        .find(|link| link["target"] == docs_uri.as_str())
        .expect("docs link should exist");
    assert_eq!(docs_link["range"]["start"]["line"], 13);
    assert_eq!(docs_link["range"]["start"]["character"], 6);
    assert_eq!(docs_link["range"]["end"]["line"], 13);
    assert_eq!(docs_link["range"]["end"]["character"], 10);

    let adrs_link = links
        .iter()
        .find(|link| link["target"] == adrs_uri.as_str())
        .expect("adrs link should exist");
    assert_eq!(adrs_link["range"]["start"]["line"], 14);
    assert_eq!(adrs_link["range"]["start"]["character"], 6);
    assert_eq!(adrs_link["range"]["end"]["line"], 14);
    assert_eq!(adrs_link["range"]["end"]["character"], 10);
}
#[tokio::test(flavor = "current_thread")]
async fn document_links_resolve_interpolated_include_paths() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let model_path = workspace_root.join("model/people-and-software-systems.dsl");
    let model_source = read_workspace_file(&model_path);
    let model_uri = file_uri_from_path(&model_path);
    open_document(&mut service, &model_uri, &model_source).await;

    let response = request_json(
        &mut service,
        "textDocument/documentLink",
        json!({
            "textDocument": { "uri": model_uri.as_str() }
        }),
    )
    .await;

    let links = response["result"]
        .as_array()
        .expect("document links should return an item array");
    assert!(
        links.is_empty(),
        "ambiguous include spans should not produce overlapping document links"
    );
}
