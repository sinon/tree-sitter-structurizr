use std::{fs, path::Path};

use serde_json::json;
use tower_lsp_server::ls_types::Uri;

use crate::support::{
    AnnotatedSource, TempWorkspace, TestService, annotated_source, file_uri, file_uri_from_path,
    initialize, initialize_with_workspace_folders, initialized, new_service, open_document,
    read_workspace_file, request_json, workspace_fixture_path,
};

use super::shared::{
    ARCHETYPE_THIS_CURSOR_SOURCE, DIRECT_REFERENCES_CURSOR_SOURCE,
    HIERARCHICAL_SELECTOR_CURSOR_SOURCE, SELECTOR_SEGMENT_CURSOR_SOURCE,
    SELECTOR_THIS_CURSOR_SOURCE, THIS_SOURCE_CURSOR_SOURCE, copied_workspace_fixture,
    read_annotated_cursor_workspace_fixture,
};

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_same_document_relationship_references() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(DIRECT_REFERENCES_CURSOR_SOURCE);
    let uri = file_uri("direct-references-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.only_position(),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 5);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_prefers_selector_context_for_same_document_this_references() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(SELECTOR_THIS_CURSOR_SOURCE);
    let uri = file_uri("selector-this-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("this-reference"),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 3);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_hierarchical_selector_targets() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(HIERARCHICAL_SELECTOR_CURSOR_SOURCE);
    let uri = file_uri("hierarchical-selector-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("selector-target"),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 5);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_same_document_dotted_hierarchical_references() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(HIERARCHICAL_SELECTOR_CURSOR_SOURCE);
    let uri = file_uri("hierarchical-selector-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("dotted-reference"),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 5);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_each_selector_segment_to_its_own_binding() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(SELECTOR_SEGMENT_CURSOR_SOURCE);
    let uri = file_uri("selector-segments.dsl");
    open_document(&mut service, &uri, source.source()).await;

    for (marker, expected_line, expected_character) in [
        ("selector-system", 6, 8),
        ("selector-api", 7, 12),
        ("selector-worker", 8, 16),
    ] {
        let response = request_json(
            &mut service,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": source.position(marker),
            }),
        )
        .await;

        assert_eq!(response["result"]["uri"], uri.as_str(), "{marker}");
        assert_eq!(
            response["result"]["range"]["start"]["line"], expected_line,
            "{marker}"
        );
        assert_eq!(
            response["result"]["range"]["start"]["character"], expected_character,
            "{marker}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_selector_segments_from_the_full_selector_path() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(
        r#"workspace {
    model {
        !identifiers hierarchical

        live = softwareSystem "Live"

        <CURSOR:env-declaration>live = deploymentEnvironment "Live" {
            aws = deploymentNode "AWS" {
                region = infrastructureNode "Region"
            }
        }

        !element <CURSOR:selector-live>live.aws.region {
            properties {
                "team" "Ops"
            }
        }
    }
}
"#,
    );
    let uri = file_uri("selector-ambiguous-prefix.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("selector-live"),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 6);
    assert_eq!(response["result"]["range"]["start"]["character"], 8);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_for_this_source_uses_the_binding_span() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(THIS_SOURCE_CURSOR_SOURCE);
    let uri = file_uri("this-source-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("this-source"),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 5);
    assert_eq!(response["result"]["range"]["start"]["character"], 16);
    assert_eq!(response["result"]["range"]["end"]["line"], 5);
    assert_eq!(response["result"]["range"]["end"]["character"], 20);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_for_archetype_backed_this_uses_the_nearest_owner() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(ARCHETYPE_THIS_CURSOR_SOURCE);
    let uri = file_uri("archetype-this-ok.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": source.position("this-source"),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 12);
    assert_eq!(response["result"]["range"]["start"]["character"], 16);
    assert_eq!(response["result"]["range"]["end"]["line"], 12);
    assert_eq!(response["result"]["range"]["end"]["character"], 34);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_view_scope_references() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("cross-file-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let views_path = workspace_root.join("views.dsl");
    let views_source = read_annotated_cursor_workspace_fixture("cross-file-navigation/views.dsl");
    let views_uri = file_uri_from_path(&views_path);
    open_document(&mut service, &views_uri, views_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": views_uri.as_str() },
            "position": views_source.only_position(),
        }),
    )
    .await;

    let model_uri = file_uri_from_path(&workspace_root.join("model.dsl"));
    assert_eq!(response["result"]["uri"], model_uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 2);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_big_bank_relationship_endpoints() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source =
        read_annotated_cursor_workspace_fixture("big-bank-plc/internet-banking-system.dsl");
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, document_source.source()).await;

    let customer_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));
    let web_application_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));

    for expectation in [
        DefinitionExpectation {
            marker: "customer-relationship",
            expected_uri: customer_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            marker: "web-application-relationship",
            expected_uri: web_application_uri.as_str(),
            expected_line: 2,
        },
    ] {
        assert_definition_target(&mut service, &document_uri, &document_source, expectation).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_big_bank_view_include_and_animation_references() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source =
        read_annotated_cursor_workspace_fixture("big-bank-plc/internet-banking-system.dsl");
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, document_source.source()).await;

    let people_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));
    let details_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));

    for expectation in [
        DefinitionExpectation {
            marker: "include-customer",
            expected_uri: people_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            marker: "container-animation-web",
            expected_uri: details_uri.as_str(),
            expected_line: 2,
        },
    ] {
        assert_definition_target(&mut service, &document_uri, &document_source, expectation).await;
    }

    let animation_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": document_source.position("animation-spa"),
        }),
    )
    .await;

    assert_eq!(animation_response["result"]["uri"], document_uri.as_str());
    let animation_line = animation_response["result"]["range"]["start"]["line"]
        .as_u64()
        .expect("definition line should be numeric");
    assert!(
        matches!(animation_line, 31 | 32),
        "expected animation definition to land near the declaration, got line {animation_line}"
    );

    let animation_web_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": document_source.position("animation-web"),
        }),
    )
    .await;

    assert_eq!(
        animation_web_response["result"]["uri"],
        document_uri.as_str()
    );
    let animation_web_line = animation_web_response["result"]["range"]["start"]["line"]
        .as_u64()
        .expect("definition line should be numeric");
    assert!(
        matches!(animation_web_line, 35 | 36),
        "expected animation definition to land near the declaration, got line {animation_web_line}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_big_bank_dynamic_view_references() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source =
        read_annotated_cursor_workspace_fixture("big-bank-plc/internet-banking-system.dsl");
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, document_source.source()).await;

    let details_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));

    for expectation in [
        DefinitionExpectation {
            marker: "dynamic-view",
            expected_uri: details_uri.as_str(),
            expected_line: 3,
        },
        DefinitionExpectation {
            marker: "single-page-application",
            expected_uri: details_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            marker: "signin-controller",
            expected_uri: details_uri.as_str(),
            expected_line: 4,
        },
        DefinitionExpectation {
            marker: "security-component",
            expected_uri: details_uri.as_str(),
            expected_line: 7,
        },
    ] {
        assert_definition_target(&mut service, &document_uri, &document_source, expectation).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_docs_and_adrs_path_arguments() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let details_path = workspace_root.join("model/internet-banking-system/details.dsl");
    let details_source = read_annotated_cursor_workspace_fixture(
        "big-bank-plc/model/internet-banking-system/details.dsl",
    );
    let details_uri = file_uri_from_path(&details_path);
    open_document(&mut service, &details_uri, details_source.source()).await;

    let docs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": details_uri.as_str() },
            "position": details_source.position("docs"),
        }),
    )
    .await;
    let docs_uri = file_uri_from_path(
        &details_path
            .parent()
            .expect("details path should have a parent")
            .join("docs/01-context.md"),
    );
    assert_eq!(docs_response["result"]["uri"], docs_uri.as_str());
    assert_eq!(docs_response["result"]["range"]["start"]["line"], 0);

    let adrs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": details_uri.as_str() },
            "position": details_source.position("adrs"),
        }),
    )
    .await;
    let adrs_uri = file_uri_from_path(
        &details_path
            .parent()
            .expect("details path should have a parent")
            .join("adrs/0001-record-architecture-decisions.md"),
    );
    assert_eq!(adrs_response["result"]["uri"], adrs_uri.as_str());
    assert_eq!(adrs_response["result"]["range"]["start"]["line"], 0);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_include_path_arguments() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let model_path = workspace_root.join("model/people-and-software-systems.dsl");
    let model_source = read_annotated_cursor_workspace_fixture(
        "big-bank-plc/model/people-and-software-systems.dsl",
    );
    let model_uri = file_uri_from_path(&model_path);
    open_document(&mut service, &model_uri, model_source.source()).await;

    let include_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": model_uri.as_str() },
            "position": model_source.only_position(),
        }),
    )
    .await;
    let include_results = include_response["result"]
        .as_array()
        .expect("include path definitions should return an item array");
    let details_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));
    let summary_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/summary.dsl"));
    assert!(
        include_results
            .iter()
            .any(|location| location["uri"] == details_uri.as_str())
    );
    assert!(
        include_results
            .iter()
            .any(|location| location["uri"] == summary_uri.as_str())
    );
}
#[tokio::test(flavor = "current_thread")]
async fn goto_definition_ignores_docs_and_adrs_importer_arguments() {
    let temp_workspace = TempWorkspace::new(
        "directive-importers",
        "workspace {\n  !docs docs com.example.documentation.CustomDocumentationImporter\n  !adrs decisions com.example.documentation.CustomDecisionImporter\n}\n",
        &[Path::new("docs"), Path::new("decisions")],
        &[
            (Path::new("docs/01-context.md"), "# Context"),
            (Path::new("decisions/0001-record.md"), "# Decision"),
        ],
    );
    let (mut service, _socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let workspace_path = temp_workspace.path().join("workspace.dsl");
    let workspace_source = annotated_source(
        &read_workspace_file(&workspace_path)
            .replacen(
                "com.example.documentation.CustomDocumentationImporter",
                "<CURSOR:docs-importer>com.example.documentation.CustomDocumentationImporter",
                1,
            )
            .replacen(
                "com.example.documentation.CustomDecisionImporter",
                "<CURSOR:adrs-importer>com.example.documentation.CustomDecisionImporter",
                1,
            )
            .replacen("!docs docs", "!docs <CURSOR:docs-path>docs", 1)
            .replacen("!adrs decisions", "!adrs <CURSOR:adrs-path>decisions", 1),
    );
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let docs_importer_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("docs-importer"),
        }),
    )
    .await;
    assert!(docs_importer_response["result"].is_null());

    let adrs_importer_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("adrs-importer"),
        }),
    )
    .await;
    assert!(adrs_importer_response["result"].is_null());

    let docs_path_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("docs-path"),
        }),
    )
    .await;
    let docs_uri = file_uri_from_path(
        &fs::canonicalize(temp_workspace.path().join("docs/01-context.md"))
            .expect("docs file should canonicalize"),
    );
    assert_eq!(docs_path_response["result"]["uri"], docs_uri.as_str());

    let adrs_path_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("adrs-path"),
        }),
    )
    .await;
    let adrs_uri = file_uri_from_path(
        &fs::canonicalize(temp_workspace.path().join("decisions/0001-record.md"))
            .expect("adrs file should canonicalize"),
    );
    assert_eq!(adrs_path_response["result"]["uri"], adrs_uri.as_str());
}
#[tokio::test(flavor = "current_thread")]
async fn goto_definition_returns_no_result_for_empty_docs_and_adrs_directories() {
    let temp_workspace = TempWorkspace::new(
        "empty-path-targets",
        "!docs docs\n!adrs adrs\n",
        &[Path::new("docs"), Path::new("adrs")],
        &[],
    );
    let (mut service, _socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let workspace_path = temp_workspace.path().join("workspace.dsl");
    let workspace_source = annotated_source(
        &read_workspace_file(&workspace_path)
            .replacen("!docs docs", "!docs <CURSOR:docs>docs", 1)
            .replacen("!adrs adrs", "!adrs <CURSOR:adrs>adrs", 1),
    );
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let docs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("docs"),
        }),
    )
    .await;
    assert!(docs_response["result"].is_null());

    let adrs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("adrs"),
        }),
    )
    .await;
    assert!(adrs_response["result"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_uses_direct_child_docs_files_only() {
    let temp_workspace = TempWorkspace::new(
        "direct-child-docs",
        "!docs docs\n",
        &[Path::new("docs/nested")],
        &[
            (Path::new("docs/01-top.md"), "# Top"),
            (Path::new("docs/nested/02-nested.md"), "# Nested"),
        ],
    );
    let (mut service, _socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let workspace_path = temp_workspace.path().join("workspace.dsl");
    let workspace_source = annotated_source(&read_workspace_file(&workspace_path).replacen(
        "!docs docs",
        "!docs <CURSOR>docs",
        1,
    ));
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let docs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.only_position(),
        }),
    )
    .await;
    let docs_uri = file_uri_from_path(
        &fs::canonicalize(temp_workspace.path().join("docs/01-top.md"))
            .expect("direct child docs file should canonicalize"),
    );
    assert_eq!(docs_response["result"]["uri"], docs_uri.as_str());
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_deployment_instance_targets() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    for expectation in [
        DefinitionExpectation {
            marker: "api-target",
            expected_uri: workspace_uri.as_str(),
            expected_line: 3,
        },
        DefinitionExpectation {
            marker: "system-target",
            expected_uri: workspace_uri.as_str(),
            expected_line: 2,
        },
    ] {
        assert_definition_target(&mut service, &workspace_uri, &workspace_source, expectation)
            .await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_deployment_relationship_endpoints() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    for expectation in [
        DefinitionExpectation {
            marker: "primary-relationship",
            expected_uri: workspace_uri.as_str(),
            expected_line: 7,
        },
        DefinitionExpectation {
            marker: "gateway-relationship",
            expected_uri: workspace_uri.as_str(),
            expected_line: 8,
        },
        DefinitionExpectation {
            marker: "secondary-api-instance-relationship",
            expected_uri: workspace_uri.as_str(),
            expected_line: 15,
        },
    ] {
        assert_definition_target(&mut service, &workspace_uri, &workspace_source, expectation)
            .await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_deployment_this_to_the_enclosing_instance() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    assert_definition_target(
        &mut service,
        &workspace_uri,
        &workspace_source,
        DefinitionExpectation {
            marker: "deferred-this",
            expected_uri: workspace_uri.as_str(),
            expected_line: 9,
        },
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_big_bank_instance_targets() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source =
        read_annotated_cursor_workspace_fixture("big-bank-plc/internet-banking-system.dsl");
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, document_source.source()).await;

    let web_application_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));
    let mainframe_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));

    for expectation in [
        DefinitionExpectation {
            marker: "web-application-instance",
            expected_uri: web_application_uri.as_str(),
            expected_line: 2,
        },
        DefinitionExpectation {
            marker: "mainframe-instance",
            expected_uri: mainframe_uri.as_str(),
            expected_line: 6,
        },
    ] {
        assert_definition_target(&mut service, &document_uri, &document_source, expectation).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn goto_type_definition_resolves_instance_declarations_to_model_elements() {
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
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("api-instance-declaration"),
        }),
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
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("secondary-api-instance-relationship"),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], workspace_uri.as_str());
    assert_eq!(response["result"]["range"]["start"]["line"], 3);
}

#[tokio::test(flavor = "current_thread")]
async fn goto_type_definition_resolves_cross_file_big_bank_instance_declarations() {
    let (mut service, _socket) = new_service();
    let temp_workspace = copied_workspace_fixture("big-bank-plc");
    let workspace_root = fs::canonicalize(temp_workspace.path())
        .expect("copied big-bank workspace path should canonicalize");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source =
        read_annotated_cursor_workspace_fixture("big-bank-plc/internet-banking-system.dsl");
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, document_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": document_source.position("live-primary-database-instance"),
        }),
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
    let workspace_source =
        read_annotated_cursor_workspace_fixture("deployment-navigation/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/typeDefinition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.position("primary-deployment-node"),
        }),
    )
    .await;

    assert!(response["result"].is_null());
}
#[tokio::test(flavor = "current_thread")]
async fn goto_definition_returns_no_result_for_duplicate_bindings() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("duplicate-bindings");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source =
        read_annotated_cursor_workspace_fixture("duplicate-bindings/workspace.dsl");
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": workspace_source.only_position(),
        }),
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
    let view_source =
        read_annotated_cursor_workspace_fixture("multi-instance-open-fragment/shared/view.dsl");
    let view_uri = file_uri_from_path(&view_path);
    open_document(&mut service, &view_uri, view_source.source()).await;

    let response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": view_uri.as_str() },
            "position": view_source.only_position(),
        }),
    )
    .await;

    assert!(response["result"].is_null());
}
struct DefinitionExpectation<'a> {
    marker: &'a str,
    expected_uri: &'a str,
    expected_line: u64,
}

async fn assert_definition_target(
    service: &mut TestService,
    document_uri: &Uri,
    document_source: &AnnotatedSource,
    expectation: DefinitionExpectation<'_>,
) {
    let response = request_json(
        service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": document_source.position(expectation.marker),
        }),
    )
    .await;

    assert_eq!(response["result"]["uri"], expectation.expected_uri);
    assert_eq!(
        response["result"]["range"]["start"]["line"],
        expectation.expected_line
    );
}
