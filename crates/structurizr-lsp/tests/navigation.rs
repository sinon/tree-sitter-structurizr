mod support;

use std::{fs, path::Path};

use serde_json::json;
use support::{
    close_document, file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders,
    initialized, new_service, next_publish_diagnostics_for_uri, open_document, position_in,
    request_json, workspace_fixture_path,
};

const DIRECT_REFERENCES_SOURCE: &str =
    include_str!("../../../tests/fixtures/lsp/relationships/named-relationships-ok.dsl");
const COMPLETION_SOURCE: &str = "workspace {\n  !i\n}\n";

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
        2,
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
async fn completion_returns_directive_keywords_for_prefixes() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("completion.dsl");
    open_document(&mut service, &uri, COMPLETION_SOURCE).await;

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
        .map(|item| {
            item["label"]
                .as_str()
                .expect("completion label should be a string")
        })
        .collect::<Vec<_>>();

    assert!(labels.contains(&"!include"));
    assert!(labels.contains(&"!identifiers"));
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_same_document_relationship_references() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, DIRECT_REFERENCES_SOURCE).await;

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
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, DIRECT_REFERENCES_SOURCE).await;

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

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_view_scope_references() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("cross-file-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let views_path = workspace_root.join("views.dsl");
    let views_source = read_workspace_file(&views_path);
    let views_uri = file_uri_from_path(&views_path);
    open_document(&mut service, &views_uri, &views_source).await;

    let position = position_in(&views_source, "systemContext system", 15);
    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": views_uri.as_str() },
            "position": position,
        }),
        6,
    )
    .await;

    let model_uri = file_uri_from_path(&workspace_root.join("model.dsl"));
    assert_eq!(response["result"]["uri"], model_uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 2);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_big_bank_relationship_endpoints() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source = read_workspace_file(&document_path);
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;

    let customer_position = position_in(&document_source, "customer -> webApplication", 1);
    let customer_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": customer_position,
        }),
        10,
    )
    .await;

    let customer_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));
    assert_eq!(customer_response["result"]["uri"], customer_uri.as_str());
    assert_eq!(customer_response["result"]["range"]["start"]["line"], 0);

    let web_application_position = position_in(&document_source, "customer -> webApplication", 13);
    let web_application_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": web_application_position,
        }),
        11,
    )
    .await;

    let web_application_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));
    assert_eq!(
        web_application_response["result"]["uri"],
        web_application_uri.as_str()
    );
    assert_eq!(
        web_application_response["result"]["range"]["start"]["line"],
        2
    );
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_deployment_instance_targets() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let api_target_position = position_in(&workspace_source, "containerInstance api", 18);
    let api_target_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": api_target_position,
        }),
        12,
    )
    .await;

    assert_eq!(api_target_response["result"]["uri"], workspace_uri.as_str());
    assert_eq!(api_target_response["result"]["range"]["start"]["line"], 3);

    let system_target_position =
        position_in(&workspace_source, "softwareSystemInstance system", 23);
    let system_target_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": system_target_position,
        }),
        13,
    )
    .await;

    assert_eq!(
        system_target_response["result"]["uri"],
        workspace_uri.as_str()
    );
    assert_eq!(
        system_target_response["result"]["range"]["start"]["line"],
        2
    );
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_deployment_relationship_endpoints() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let primary_position = position_in(&workspace_source, "primary -> gateway", 1);
    let primary_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": primary_position,
        }),
        14,
    )
    .await;
    assert_eq!(primary_response["result"]["uri"], workspace_uri.as_str());
    assert_eq!(primary_response["result"]["range"]["start"]["line"], 7);

    let gateway_instance_body_position = position_in(&workspace_source, "gateway -> this", 1);
    let gateway_instance_body_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": gateway_instance_body_position,
        }),
        15,
    )
    .await;
    assert_eq!(
        gateway_instance_body_response["result"]["uri"],
        workspace_uri.as_str()
    );
    assert_eq!(
        gateway_instance_body_response["result"]["range"]["start"]["line"],
        8
    );

    let api_instance_position = position_in(&workspace_source, "gateway -> apiInstance", 11);
    let api_instance_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": api_instance_position,
        }),
        16,
    )
    .await;
    assert_eq!(
        api_instance_response["result"]["uri"],
        workspace_uri.as_str()
    );
    assert_eq!(api_instance_response["result"]["range"]["start"]["line"], 9);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_returns_no_result_for_deferred_deployment_this() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let this_position = position_in(&workspace_source, "gateway -> this", 11);
    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": this_position,
        }),
        17,
    )
    .await;

    assert!(response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_big_bank_instance_targets() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source = read_workspace_file(&document_path);
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;

    let web_application_target_position =
        position_in(&document_source, "containerInstance webApplication", 18);
    let web_application_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": web_application_target_position,
        }),
        18,
    )
    .await;

    let web_application_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));
    assert_eq!(
        web_application_response["result"]["uri"],
        web_application_uri.as_str()
    );
    assert_eq!(
        web_application_response["result"]["range"]["start"]["line"],
        2
    );

    let mainframe_target_position =
        position_in(&document_source, "softwareSystemInstance mainframe", 23);
    let mainframe_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": mainframe_target_position,
        }),
        19,
    )
    .await;

    let mainframe_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));
    assert_eq!(mainframe_response["result"]["uri"], mainframe_uri.as_str());
    assert_eq!(mainframe_response["result"]["range"]["start"]["line"], 6);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_type_definition_resolves_instance_declarations_to_model_elements() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let position = position_in(&workspace_source, "apiInstance = containerInstance api", 1);
    let response = request_json(
        &mut service,
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": position,
        }),
        22,
    )
    .await;

    assert_eq!(response["result"]["uri"], workspace_uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 3);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_type_definition_resolves_instance_references_to_model_elements() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let position = position_in(&workspace_source, "gateway -> apiInstance", 11);
    let response = request_json(
        &mut service,
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": position,
        }),
        23,
    )
    .await;

    assert_eq!(response["result"]["uri"], workspace_uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 3);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_type_definition_resolves_cross_file_big_bank_instance_declarations() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source = read_workspace_file(&document_path);
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;

    let position = position_in(
        &document_source,
        "livePrimaryDatabaseInstance = containerInstance database",
        1,
    );
    let response = request_json(
        &mut service,
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": position,
        }),
        24,
    )
    .await;

    let database_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));
    assert_eq!(response["result"]["uri"], database_uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 11);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_type_definition_returns_no_result_for_plain_deployment_nodes() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let position = position_in(&workspace_source, "primary = deploymentNode", 1);
    let response = request_json(
        &mut service,
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": position,
        }),
        25,
    )
    .await;

    assert!(response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn references_follow_instance_targets_from_model_declarations() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let position = position_in(&workspace_source, "api = container", 1);
    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": position,
            "context": { "includeDeclaration": true },
        }),
        20,
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

    assert_eq!(lines, vec![3, 9]);
}

#[tokio::test(flavor = "current_thread")]
async fn references_follow_deployment_symbols_from_relationship_endpoints() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let position = position_in(&workspace_source, "gateway = infrastructureNode", 1);
    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": position,
            "context": { "includeDeclaration": true },
        }),
        21,
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

    assert_eq!(lines, vec![8, 10, 15, 16]);
}

#[tokio::test(flavor = "current_thread")]
async fn references_follow_cross_file_bindings_from_model_declarations() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("cross-file-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let model_path = workspace_root.join("model.dsl");
    let model_source = read_workspace_file(&model_path);
    let model_uri = file_uri_from_path(&model_path);
    open_document(&mut service, &model_uri, &model_source).await;

    let position = position_in(&model_source, "system =", 1);
    let response = request_json(
        &mut service,
        "textDocument/references",
        json!({
            "textDocument": { "uri": model_uri.as_str() },
            "position": position,
            "context": { "includeDeclaration": true },
        }),
        7,
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

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_returns_no_result_for_duplicate_bindings() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("duplicate-bindings");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let position = position_in(&workspace_source, "user -> api", 8);
    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": position,
        }),
        8,
    )
    .await;

    assert!(response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_returns_no_result_for_multi_instance_open_fragments() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("multi-instance-open-fragment");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let view_path = workspace_root.join("shared/view.dsl");
    let view_source = read_workspace_file(&view_path);
    let view_uri = file_uri_from_path(&view_path);
    open_document(&mut service, &view_uri, &view_source).await;

    let position = position_in(&view_source, "include api", 8);
    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": view_uri.as_str() },
            "position": position,
        }),
        9,
    )
    .await;

    assert!(response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn diagnostics_publish_bounded_semantic_errors() {
    let (mut service, mut socket) = new_service();
    let workspace_root = workspace_fixture_path("duplicate-bindings");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let alpha_path = workspace_root.join("alpha.dsl");
    let alpha_uri = file_uri_from_path(&alpha_path);
    open_document(&mut service, &alpha_uri, &read_workspace_file(&alpha_path)).await;
    let alpha_notification =
        next_publish_diagnostics_for_uri(&mut socket, alpha_uri.as_str()).await;

    let alpha_messages = alpha_notification["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics notification should include an array")
        .iter()
        .map(|diagnostic| {
            diagnostic["message"]
                .as_str()
                .expect("diagnostic message should be a string")
        })
        .collect::<Vec<_>>();
    assert_eq!(alpha_messages, vec!["duplicate element binding: api"]);

    close_document(&mut service, &alpha_uri).await;
    let _ = next_publish_diagnostics_for_uri(&mut socket, alpha_uri.as_str()).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(
        &mut service,
        &workspace_uri,
        &read_workspace_file(&workspace_path),
    )
    .await;
    let workspace_notification =
        next_publish_diagnostics_for_uri(&mut socket, workspace_uri.as_str()).await;

    let workspace_messages = workspace_notification["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics notification should include an array")
        .iter()
        .map(|diagnostic| {
            diagnostic["message"]
                .as_str()
                .expect("diagnostic message should be a string")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        workspace_messages,
        vec!["ambiguous identifier reference: api"]
    );
}

fn read_workspace_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "failed to read workspace file `{}`: {error}",
            path.display()
        )
    })
}
