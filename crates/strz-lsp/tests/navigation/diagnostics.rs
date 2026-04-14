use std::path::Path;

use crate::support::{
    TempWorkspace, annotated_source, change_document, close_document, file_uri_from_path,
    initialize_with_workspace_folders, initialized, new_service, next_publish_diagnostics_for_uri,
    open_document, read_workspace_file, workspace_fixture_path,
};

#[tokio::test(flavor = "current_thread")]
async fn assigned_deployment_environment_after_partial_relationship_suppresses_only_recovery_equals_diagnostic()
 {
    let temp_workspace = TempWorkspace::new(
        "relationship-completion-before-assigned-deployment-environment",
        "workspace {\n  !include model.dsl\n  !include relationships.dsl\n}\n",
        &[],
        &[
            (
                Path::new("model.dsl"),
                "model {\n  customer = person \"Customer\"\n  webApplication = softwareSystem \"Web Application\"\n}\n",
            ),
            (
                Path::new("relationships.dsl"),
                "model {\n  customer -> webApplication \"Uses\"\n\n  env = deploymentEnvironment \"Development\" {\n  }\n}\n",
            ),
        ],
    );
    let (mut service, mut socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let document_path = temp_workspace.path().join("relationships.dsl");
    let document_source = read_workspace_file(&document_path);
    let fresh_source = annotated_source(&document_source.replacen(
        "\n\n  env = deploymentEnvironment",
        "\n\n  cust<CURSOR>\n  env = deploymentEnvironment",
        1,
    ));
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;
    let _ = next_publish_diagnostics_for_uri(&mut socket, document_uri.as_str()).await;
    change_document(&mut service, &document_uri, 2, fresh_source.source()).await;
    let diagnostics_notification =
        next_publish_diagnostics_for_uri(&mut socket, document_uri.as_str()).await;
    let diagnostics = diagnostics_notification["params"]["diagnostics"]
        .as_array()
        .expect("publishDiagnostics should include a diagnostics array");

    assert!(
        diagnostics.is_empty(),
        "partial relationship source before assigned deploymentEnvironment should not publish the recovery `=` diagnostic: {diagnostics:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn assigned_deployment_environment_syntax_errors_still_publish_diagnostics_after_partial_relationship()
 {
    let temp_workspace = TempWorkspace::new(
        "relationship-errors-before-assigned-deployment-environment",
        "workspace {\n  !include model.dsl\n  !include relationships.dsl\n}\n",
        &[],
        &[
            (
                Path::new("model.dsl"),
                "model {\n  customer = person \"Customer\"\n  webApplication = softwareSystem \"Web Application\"\n}\n",
            ),
            (
                Path::new("relationships.dsl"),
                "model {\n  customer -> webApplication \"Uses\"\n\n  env = deploymentEnvironment \"Development\" {\n  }\n}\n",
            ),
        ],
    );
    let (mut service, mut socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let document_path = temp_workspace.path().join("relationships.dsl");
    let document_source = read_workspace_file(&document_path);
    let fresh_source = document_source.replacen(
        "\n\n  env = deploymentEnvironment \"Development\" {",
        "\n\n  cust\n  env = deploymentEnvironment {",
        1,
    );
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;
    let _ = next_publish_diagnostics_for_uri(&mut socket, document_uri.as_str()).await;
    change_document(&mut service, &document_uri, 2, &fresh_source).await;
    let diagnostics_notification =
        next_publish_diagnostics_for_uri(&mut socket, document_uri.as_str()).await;
    let diagnostics = diagnostics_notification["params"]["diagnostics"]
        .as_array()
        .expect("publishDiagnostics should include a diagnostics array");

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["message"] == "unexpected syntax"
                && diagnostic["range"]["start"]["line"] == 4
        }),
        "malformed assigned deploymentEnvironment syntax should still publish a diagnostic on the deployment line: {diagnostics:?}"
    );
}
#[tokio::test(flavor = "current_thread")]
async fn diagnostics_do_not_report_syntax_errors_for_docs_and_adrs_importers() {
    let temp_workspace = TempWorkspace::new(
        "directive-importer-diagnostics",
        "workspace \"Some System\" \"Description\" {\n  model {\n    contributor = person \"Person\"\n    someSystem = softwareSystem \"Some System\" {\n      !docs docs com.example.documentation.CustomDocumentationImporter\n      !adrs decisions adrtools\n      someContainer = container \"Some Container\" \"\" \"\"\n    }\n  }\n}\n",
        &[Path::new("docs"), Path::new("decisions")],
        &[
            (Path::new("docs/01-context.md"), "# Context"),
            (Path::new("decisions/0001-record.md"), "# Decision"),
        ],
    );
    let (mut service, mut socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let workspace_path = temp_workspace.path().join("workspace.dsl");
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let notification = next_publish_diagnostics_for_uri(&mut socket, workspace_uri.as_str()).await;
    let diagnostics = notification["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics notification should include an array");
    assert!(
        diagnostics.is_empty(),
        "optional importer arguments should not leave syntax errors behind: {diagnostics:?}"
    );
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
    assert_eq!(
        alpha_messages,
        vec![
            "multiple model sections are not permitted in a DSL definition",
            "duplicate element binding: api",
        ]
    );

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
