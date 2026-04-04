mod support;

use std::{collections::BTreeSet, path::Path};

use serde_json::json;
use support::{
    TempWorkspace, annotated_source, file_uri, file_uri_from_path, initialize,
    initialize_with_workspace_folders, initialized, new_service, next_publish_diagnostics_for_uri,
    open_document, read_workspace_file, request_json,
};
use tower_lsp_server::ls_types::Uri;

const FLAT_ELEMENT_RENAME_SOURCE: &str = r#"workspace {
    model {
        user = person "User"
        system = softwareSystem "Payments" {
            <CURSOR:api-declaration>api = container "Payments API"
        }
        user -> <CURSOR:api-relationship-reference>api "Uses"
    }
    views {
        container system "Payments" {
            include <CURSOR:api-view-reference>api
            exclude <CURSOR:api-view-exclude>api
            autoLayout
        }
    }
}
"#;

const HIERARCHICAL_RENAME_SOURCE: &str = r#"workspace {
    model {
        !identifiers hierarchical

        system = softwareSystem "Payments" {
            <CURSOR:api-declaration>api = container "Payments API"
        }
    }
}
"#;

const HIERARCHICAL_DEPLOYMENT_RENAME_SOURCE: &str = r#"workspace {
    !identifiers hierarchical

    model {
        system = softwareSystem "Payments" {
            api = container "API"
        }

        live = deploymentEnvironment "Live" {
            edge = deploymentNode "Edge" {
                <CURSOR:api-instance-declaration>apiInstance = containerInstance api
            }
        }
    }

    views {
        deployment system "Live" {
            include live.edge.apiInstance
            autoLayout
        }
    }
}
"#;

const DUPLICATE_BINDINGS_RENAME_SOURCE: &str = r#"workspace {
    model {
        system = softwareSystem "Payments" {
            <CURSOR:first-api>api = container "API 1"
            api = container "API 2"
        }
    }
}
"#;

#[tokio::test(flavor = "current_thread")]
async fn prepare_rename_returns_a_placeholder_for_flat_element_declarations() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(FLAT_ELEMENT_RENAME_SOURCE);
    let uri = file_uri("rename-flat-element.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response =
        request_prepare_rename(&mut service, &uri, source.position("api-declaration")).await;

    assert_eq!(response["result"]["placeholder"], "api");
    assert_eq!(response["result"]["range"]["start"]["line"], 4);
    assert_eq!(response["result"]["range"]["start"]["character"], 12);
    assert_eq!(response["result"]["range"]["end"]["line"], 4);
    assert_eq!(response["result"]["range"]["end"]["character"], 15);
}

#[tokio::test(flavor = "current_thread")]
async fn rename_rewrites_same_document_flat_element_bindings_and_references() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(FLAT_ELEMENT_RENAME_SOURCE);
    let uri = file_uri("rename-flat-element.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_rename(
        &mut service,
        &uri,
        source.position("api-view-exclude"),
        "paymentsApi",
    )
    .await;

    assert_workspace_edit(&response, &[(&uri, &[4, 6, 10, 11])], "paymentsApi");
}

#[tokio::test(flavor = "current_thread")]
async fn rename_rewrites_cross_file_container_instance_bindings_and_references() {
    let temp_workspace = TempWorkspace::new(
        "rename-cross-file-container-instance",
        "workspace {\n  !include model.dsl\n  !include deployment.dsl\n  !include views.dsl\n}\n",
        &[],
        &[
            (
                Path::new("model.dsl"),
                "model {\n  system = softwareSystem \"Payments\" {\n    api = container \"API\"\n  }\n}\n",
            ),
            (
                Path::new("deployment.dsl"),
                "model {\n  deploymentEnvironment \"Prod\" {\n    edge = deploymentNode \"Edge\" {\n      apiInstance = containerInstance api\n    }\n\n    edge -> apiInstance \"Routes\"\n  }\n}\n",
            ),
            (
                Path::new("views.dsl"),
                "views {\n  deployment system \"Prod\" {\n    include edge\n    include apiInstance\n    autoLayout\n  }\n}\n",
            ),
        ],
    );
    let (mut service, mut socket) = new_service();
    let workspace_root = temp_workspace
        .path()
        .canonicalize()
        .expect("temp workspace root should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let views_path = workspace_root.join("views.dsl");
    let views_source = annotated_source(&read_workspace_file(&views_path).replacen(
        "include apiInstance",
        "include <CURSOR:api-instance>apiInstance",
        1,
    ));
    let views_uri = file_uri_from_path(&views_path);
    open_document(&mut service, &views_uri, views_source.source()).await;
    let _ = next_publish_diagnostics_for_uri(&mut socket, views_uri.as_str()).await;

    let response = request_rename(
        &mut service,
        &views_uri,
        views_source.position("api-instance"),
        "paymentsApi",
    )
    .await;

    let deployment_uri = file_uri_from_path(&workspace_root.join("deployment.dsl"));
    assert_workspace_edit(
        &response,
        &[(&deployment_uri, &[3, 6]), (&views_uri, &[3])],
        "paymentsApi",
    );
}

#[tokio::test(flavor = "current_thread")]
async fn rename_returns_no_result_when_element_identifiers_are_hierarchical() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(HIERARCHICAL_RENAME_SOURCE);
    let uri = file_uri("rename-hierarchical.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_rename(
        &mut service,
        &uri,
        source.position("api-declaration"),
        "paymentsApi",
    )
    .await;

    assert!(response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn rename_returns_no_result_when_deployment_identifiers_are_hierarchical() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(HIERARCHICAL_DEPLOYMENT_RENAME_SOURCE);
    let uri = file_uri("rename-hierarchical-deployment.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_rename(
        &mut service,
        &uri,
        source.position("api-instance-declaration"),
        "paymentsApi",
    )
    .await;

    assert!(response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn rename_returns_no_result_for_duplicate_bindings() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(DUPLICATE_BINDINGS_RENAME_SOURCE);
    let uri = file_uri("rename-duplicate-bindings.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_rename(
        &mut service,
        &uri,
        source.position("first-api"),
        "paymentsApi",
    )
    .await;

    assert!(response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn rename_rejects_dotted_new_names_for_the_flat_slice() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(FLAT_ELEMENT_RENAME_SOURCE);
    let uri = file_uri("rename-invalid-new-name.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_rename(
        &mut service,
        &uri,
        source.position("api-declaration"),
        "payments.api",
    )
    .await;

    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(
        response["error"]["message"],
        "rename newName must match the supported flat Structurizr identifier shape"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn rename_returns_no_result_when_workspace_instances_disagree_on_edit_sets() {
    let temp_workspace = TempWorkspace::new(
        "rename-multi-instance-disagreement",
        "workspace {\n  !include shared/model.dsl\n  !include alpha/views.dsl\n}\n",
        &[Path::new("shared"), Path::new("alpha"), Path::new("beta")],
        &[
            (
                Path::new("beta.dsl"),
                "workspace {\n  !include shared/model.dsl\n  !include beta/views.dsl\n}\n",
            ),
            (
                Path::new("shared/model.dsl"),
                "model {\n  system = softwareSystem \"Payments\" {\n    api = container \"API\"\n  }\n}\n",
            ),
            (
                Path::new("alpha/views.dsl"),
                "views {\n  container system \"Payments\" {\n    include api\n    autoLayout\n  }\n}\n",
            ),
            (
                Path::new("beta/views.dsl"),
                "views {\n  container system \"Payments\" {\n    include api\n    autoLayout\n  }\n}\n",
            ),
        ],
    );
    let (mut service, mut socket) = new_service();
    let workspace_root = temp_workspace
        .path()
        .canonicalize()
        .expect("temp workspace root should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let model_path = workspace_root.join("shared/model.dsl");
    let model_source = annotated_source(&read_workspace_file(&model_path).replacen(
        "api = container",
        "<CURSOR:api-declaration>api = container",
        1,
    ));
    let model_uri = file_uri_from_path(&model_path);
    open_document(&mut service, &model_uri, model_source.source()).await;
    let _ = next_publish_diagnostics_for_uri(&mut socket, model_uri.as_str()).await;

    let response = request_rename(
        &mut service,
        &model_uri,
        model_source.position("api-declaration"),
        "paymentsApi",
    )
    .await;

    assert!(response["result"].is_null());
}

async fn request_prepare_rename(
    service: &mut support::TestService,
    uri: &Uri,
    position: tower_lsp_server::ls_types::Position,
) -> serde_json::Value {
    request_json(
        service,
        "textDocument/prepareRename",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
    )
    .await
}

async fn request_rename(
    service: &mut support::TestService,
    uri: &Uri,
    position: tower_lsp_server::ls_types::Position,
    new_name: &str,
) -> serde_json::Value {
    request_json(
        service,
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
            "newName": new_name,
        }),
    )
    .await
}

fn assert_workspace_edit(
    response: &serde_json::Value,
    expected_documents: &[(&Uri, &[u64])],
    expected_text: &str,
) {
    let changes = response["result"]["changes"]
        .as_object()
        .expect("rename should return workspace edits");
    let actual_uris = changes.keys().cloned().collect::<BTreeSet<_>>();
    let expected_uris = expected_documents
        .iter()
        .map(|(uri, _)| uri.as_str().to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(actual_uris, expected_uris);

    for (uri, expected_lines) in expected_documents {
        let edits = changes[uri.as_str()]
            .as_array()
            .unwrap_or_else(|| panic!("expected edits for `{}`", uri.as_str()));
        let lines = edits
            .iter()
            .map(|edit| {
                edit["range"]["start"]["line"]
                    .as_u64()
                    .expect("edit start line should be a number")
            })
            .collect::<Vec<_>>();
        let new_texts = edits
            .iter()
            .map(|edit| {
                edit["newText"]
                    .as_str()
                    .expect("edit newText should be a string")
            })
            .collect::<Vec<_>>();

        assert_eq!(lines, *expected_lines);
        assert!(
            new_texts.iter().all(|new_text| *new_text == expected_text),
            "expected all edits for `{}` to use `{expected_text}`, got {new_texts:?}",
            uri.as_str()
        );
    }
}
