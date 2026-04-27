use serde_json::json;

use crate::support::{
    annotated_source, file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders,
    initialized, new_service, open_document, request_json, workspace_fixture_path,
};

use super::shared::{
    DIRECT_REFERENCES_CURSOR_SOURCE, HIERARCHICAL_SELECTOR_CURSOR_SOURCE,
    SELECTOR_THIS_CURSOR_SOURCE, read_annotated_cursor_workspace_fixture,
};

#[tokio::test(flavor = "current_thread")]
async fn references_include_definition_when_requested() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(DIRECT_REFERENCES_CURSOR_SOURCE);
    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.only_position(),
            "context": { "includeDeclaration": true },
        }),
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

#[tokio::test(flavor = "current_thread")]
async fn references_include_same_document_selector_scoped_this_sites() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(SELECTOR_THIS_CURSOR_SOURCE);
    let uri = file_uri("selector-this-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("api-declaration"),
            "context": { "includeDeclaration": true },
        }),
    )
    .await;

    let lines = response["result"]
        .as_array()
        .expect("references should return an array")
        .iter()
        .map(|location| {
            location["range"]["start"]["line"]
                .as_u64()
                .expect("reference location line should be numeric")
        })
        .collect::<Vec<_>>();

    assert_eq!(lines, vec![3, 6, 7]);
}

#[tokio::test(flavor = "current_thread")]
async fn references_include_hierarchical_selector_targets_and_dotted_sites() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(HIERARCHICAL_SELECTOR_CURSOR_SOURCE);
    let uri = file_uri("hierarchical-selector-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("api-declaration"),
            "context": { "includeDeclaration": true },
        }),
    )
    .await;

    let lines = response["result"]
        .as_array()
        .expect("references should return an array")
        .iter()
        .map(|location| {
            location["range"]["start"]["line"]
                .as_u64()
                .expect("reference location line should be numeric")
        })
        .collect::<Vec<_>>();

    assert_eq!(lines, vec![5, 8, 9, 12]);
}

#[tokio::test(flavor = "current_thread")]
async fn references_follow_instance_targets_from_model_declarations() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("api-declaration"),
            "context": { "includeDeclaration": true },
        }),
    )
    .await;

    let lines = response["result"]
        .as_array()
        .expect("references should return an array")
        .iter()
        .map(|location| {
            location["range"]["start"]["line"]
                .as_u64()
                .expect("reference location line should be numeric")
        })
        .collect::<Vec<_>>();

    assert_eq!(lines, vec![3, 9, 15]);
}

#[tokio::test(flavor = "current_thread")]
async fn references_follow_deployment_symbols_from_relationship_endpoints() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("gateway-declaration"),
            "context": { "includeDeclaration": true },
        }),
    )
    .await;

    let lines = response["result"]
        .as_array()
        .expect("references should return an array")
        .iter()
        .map(|location| {
            location["range"]["start"]["line"]
                .as_u64()
                .expect("reference location line should be numeric")
        })
        .collect::<Vec<_>>();

    assert_eq!(lines, vec![8, 10, 19]);
}

#[tokio::test(flavor = "current_thread")]
async fn references_include_contextual_this_for_deployment_instances() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("api-instance-declaration"),
            "context": { "includeDeclaration": true },
        }),
    )
    .await;

    let lines = response["result"]
        .as_array()
        .expect("references should return an array")
        .iter()
        .map(|location| {
            location["range"]["start"]["line"]
                .as_u64()
                .expect("reference location line should be numeric")
        })
        .collect::<Vec<_>>();

    assert_eq!(lines, vec![9, 10]);
}

#[tokio::test(flavor = "current_thread")]
async fn references_follow_cross_file_bindings_from_model_declarations() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("cross-file-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let model_path = workspace_root.join("model.dsl");
    let model_source = read_annotated_cursor_workspace_fixture("cross-file-navigation/model.dsl");
    let model_uri = file_uri_from_path(&model_path);
    open_document(&mut service, &model_uri, model_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": model_uri.as_str() },
            "position": model_source.only_position(),
            "context": { "includeDeclaration": true },
        }),
    )
    .await;

    let locations = response["result"]
        .as_array()
        .expect("references should return an array");
    let rendered = locations
        .iter()
        .map(|location| {
            format!(
                "{}:{}",
                location["uri"]
                    .as_str()
                    .expect("reference location URI should be a string"),
                location["range"]["start"]["line"]
                    .as_u64()
                    .expect("reference location line should be numeric")
            )
        })
        .collect::<Vec<_>>();
    let views_uri = file_uri_from_path(&workspace_root.join("views.dsl"));

    assert_eq!(
        rendered,
        vec![
            format!("{}:2", model_uri.as_str()),
            format!("{}:3", model_uri.as_str()),
            format!("{}:1", views_uri.as_str()),
        ]
    );
}
