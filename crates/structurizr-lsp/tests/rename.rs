mod support;

use std::{fs, path::Path};

use serde_json::json;
use support::{
    annotated_source, file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders,
    initialized, new_service, next_publish_diagnostics_for_uri, open_document, request_json,
};
use tempfile::TempDir;
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
        source.position("api-relationship-reference"),
        "paymentsApi",
    )
    .await;

    assert_edit_lines(&response, &uri, &[4, 6, 10], "paymentsApi");
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
    assert_edit_lines(&response, &deployment_uri, &[3, 6], "paymentsApi");
    assert_edit_lines(&response, &views_uri, &[3], "paymentsApi");
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

fn assert_edit_lines(
    response: &serde_json::Value,
    uri: &Uri,
    expected_lines: &[u64],
    expected_text: &str,
) {
    let edits = response["result"]["changes"][uri.as_str()]
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

    assert_eq!(lines, expected_lines);
    assert!(
        new_texts.iter().all(|new_text| *new_text == expected_text),
        "expected all edits for `{}` to use `{expected_text}`, got {new_texts:?}",
        uri.as_str()
    );
}

fn read_workspace_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "workspace fixture `{}` should be readable: {error}",
            path.display()
        )
    })
}

struct TempWorkspace {
    temp_dir: TempDir,
}

impl TempWorkspace {
    fn new(
        name: &str,
        workspace_source: &str,
        directories: &[&Path],
        files: &[(&Path, &str)],
    ) -> Self {
        let temp_dir = tempfile::Builder::new()
            .prefix(name)
            .tempdir()
            .expect("temp workspace should create");
        let path = temp_dir.path();

        fs::write(path.join("workspace.dsl"), workspace_source).unwrap_or_else(|error| {
            panic!(
                "failed to write temp workspace file `{}`: {error}",
                path.join("workspace.dsl").display()
            )
        });

        for directory in directories {
            let directory_path = path.join(directory);
            fs::create_dir_all(&directory_path).unwrap_or_else(|error| {
                panic!(
                    "failed to create temp workspace directory `{}`: {error}",
                    directory_path.display()
                )
            });
        }

        for (relative_path, contents) in files {
            let file_path = path.join(relative_path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|error| {
                    panic!(
                        "failed to create temp workspace parent `{}`: {error}",
                        parent.display()
                    )
                });
            }
            fs::write(&file_path, contents).unwrap_or_else(|error| {
                panic!(
                    "failed to write temp workspace file `{}`: {error}",
                    file_path.display()
                )
            });
        }

        Self { temp_dir }
    }

    fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}
