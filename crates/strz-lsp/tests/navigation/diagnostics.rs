use std::{fs, path::Path};

#[path = "../../../../tests/support/repo_local_temp_workspace.rs"]
mod repo_local_temp_workspace;

use repo_local_temp_workspace::RepoLocalTempWorkspace;
use serde_json::Value;
use tokio::time::Duration;
use tower_lsp_server::ClientSocket;

use crate::support::{
    TempWorkspace, annotated_source, change_document, close_document, file_uri_from_path,
    initialize_with_workspace_folders, initialized, new_service, next_publish_diagnostics_for_uri,
    next_server_notification_with_timeout, open_document, read_workspace_file,
    workspace_fixture_path,
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

    let alpha_diagnostics = alpha_notification["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics notification should include an array");
    let alpha_messages = alpha_diagnostics
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
    let duplicate_diagnostic = alpha_diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "semantic.duplicate-binding")
        .expect("duplicate binding diagnostic should publish");
    assert_eq!(duplicate_diagnostic["severity"], 1);
    let related_information = duplicate_diagnostic["relatedInformation"]
        .as_array()
        .expect("duplicate binding diagnostic should include related information");
    assert_eq!(related_information.len(), 1);
    assert_eq!(
        related_information[0]["message"],
        "other element binding for api is declared here"
    );
    assert_eq!(
        related_information[0]["location"]["uri"],
        file_uri_from_path(&workspace_root.join("beta.dsl")).as_str()
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
        vec!["ambiguous element reference: api (multiple bindings match)"]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn diagnostics_publish_multi_context_disagreement_warnings() {
    let (mut service, mut socket) = new_service();
    let workspace_root = workspace_fixture_path("multi-instance-open-fragment");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let shared_view_path = workspace_root.join("shared/view.dsl");
    let shared_view_uri = file_uri_from_path(&shared_view_path);
    open_document(
        &mut service,
        &shared_view_uri,
        &read_workspace_file(&shared_view_path),
    )
    .await;
    let notification =
        next_publish_diagnostics_for_uri(&mut socket, shared_view_uri.as_str()).await;
    let diagnostics = notification["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics notification should include an array");
    let warning = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "semantic.multi-context-disagreement")
        .expect("multi-context disagreement diagnostic should publish");

    assert_eq!(warning["severity"], 2);
    assert_eq!(
        warning["message"],
        "some workspace contexts report: unresolved element or relationship reference: api (no matching binding found) (reported in 1 of 2 contexts)"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn workspace_load_failures_publish_anchored_diagnostics() {
    let workspace = RepoLocalTempWorkspace::new("lsp-diagnostics", "workspace-base-missing");
    workspace.write_file(
        "workspace.dsl",
        "workspace extends \"missing-base.dsl\" {\n}\n",
    );
    let (mut service, mut socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(workspace.path())]).await;
    initialized(&mut service).await;

    let workspace_path = workspace.file_path("workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    let source = fs::read_to_string(&workspace_path).expect("workspace source should be readable");
    open_document(&mut service, &workspace_uri, &source).await;

    let notifications =
        notifications_until_diagnostics_and_load_messages(&mut socket, workspace_uri.as_str(), 0)
            .await;
    assert!(
        workspace_load_message_notifications(&notifications).is_empty(),
        "anchored load failures should surface as diagnostics, not session messages: {notifications:?}"
    );

    let diagnostics_notification = notifications
        .last()
        .expect("diagnostics notification should be collected");
    let diagnostics = diagnostics_notification["params"]["diagnostics"]
        .as_array()
        .expect("publishDiagnostics should include a diagnostics array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], "workspace.load-failure");
    assert_eq!(
        diagnostics[0]["message"],
        "workspace base does not exist: missing-base.dsl"
    );
    assert_eq!(diagnostics[0]["range"]["start"]["line"], 0);
}

#[tokio::test(flavor = "current_thread")]
async fn unanchored_workspace_load_failures_show_and_log_once() {
    let workspace = RepoLocalTempWorkspace::new("lsp-diagnostics", "missing-root");
    workspace.write_file("workspace.dsl", "workspace {\n}\n");
    let missing_root = workspace.file_path("does-not-exist");
    let (mut service, mut socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&missing_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace.file_path("workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    let source = fs::read_to_string(&workspace_path).expect("workspace source should be readable");
    open_document(&mut service, &workspace_uri, &source).await;

    let notifications =
        notifications_until_diagnostics_and_load_messages(&mut socket, workspace_uri.as_str(), 2)
            .await;
    let load_messages = workspace_load_message_notifications(&notifications);
    assert_eq!(
        load_messages
            .iter()
            .filter_map(|notification| notification["method"].as_str())
            .collect::<Vec<_>>(),
        vec!["window/showMessage", "window/logMessage"]
    );
    assert!(
        load_messages.iter().all(|notification| {
            notification["params"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("failed to load workspace root"))
        }),
        "unanchored load failures should explain the workspace-load failure: {load_messages:?}"
    );

    change_document(&mut service, &workspace_uri, 2, &source).await;
    let notifications =
        notifications_until_diagnostics_and_load_messages(&mut socket, workspace_uri.as_str(), 0)
            .await;
    assert!(
        workspace_load_message_notifications(&notifications).is_empty(),
        "unchanged unanchored load failures should not repeat show/log messages: {notifications:?}"
    );
}

async fn notifications_until_diagnostics_and_load_messages(
    socket: &mut ClientSocket,
    expected_uri: &str,
    expected_load_message_count: usize,
) -> Vec<Value> {
    let mut notifications = Vec::new();
    let mut saw_expected_diagnostics = false;

    for _ in 0..16 {
        let notification =
            next_server_notification_with_timeout(socket, Duration::from_secs(2)).await;
        let is_expected_diagnostics = notification["method"] == "textDocument/publishDiagnostics"
            && notification["params"]["uri"] == expected_uri;
        if is_expected_diagnostics {
            saw_expected_diagnostics = true;
        }
        notifications.push(notification);

        if saw_expected_diagnostics
            && workspace_load_message_notifications(&notifications).len()
                >= expected_load_message_count
        {
            return notifications;
        }
    }

    panic!(
        "did not receive diagnostics for `{expected_uri}` and {expected_load_message_count} load messages"
    );
}

fn workspace_load_message_notifications(notifications: &[Value]) -> Vec<&Value> {
    notifications
        .iter()
        .filter(|notification| {
            matches!(
                notification["method"].as_str(),
                Some("window/showMessage" | "window/logMessage")
            )
        })
        .collect()
}
