mod support;

use std::{fs, path::Path};

use serde_json::json;
use support::{
    TestService, change_document, close_document, file_uri, file_uri_from_path, initialize,
    initialize_with_workspace_folders, initialized, new_service, next_publish_diagnostics_for_uri,
    next_server_notification, open_document, position_in, request_json, workspace_fixture_path,
};
use tempfile::TempDir;
use tokio::time::{Duration, timeout};
use tower_lsp_server::ls_types::Uri;

const DIRECT_REFERENCES_SOURCE: &str =
    include_str!("fixtures/relationships/named-relationships-ok.dsl");
const INVALID_SOURCE: &str =
    include_str!("fixtures/directives/identifiers-unexpected-tokens-err.dsl");
const COMPLETION_SOURCE: &str = "workspace {\n  !i\n}\n";
const ELEMENT_STYLE_COMPLETION_SOURCE: &str = "workspace {\n  views {\n    styles {\n      element \"Person\" {\n        ba\n      }\n    }\n  }\n}\n";
const RELATIONSHIP_STYLE_COMPLETION_SOURCE: &str = "workspace {\n  views {\n    styles {\n      relationship \"Uses\" {\n        da\n      }\n    }\n  }\n}\n";
const STYLE_VALUE_COMPLETION_SOURCE: &str = "workspace {\n  views {\n    styles {\n      relationship \"Uses\" {\n        metadata de\n      }\n    }\n  }\n}\n";
const STYLE_BLOCK_END_COMPLETION_SOURCE: &str = "workspace {\n  views {\n    styles {\n      element \"Person\" {\n        background #ffffff\n      }\n      !d\n    }\n  }\n}\n";

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
async fn completion_inside_element_style_suggests_element_style_properties() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("element-style-completion.dsl");
    open_document(&mut service, &uri, ELEMENT_STYLE_COMPLETION_SOURCE).await;

    let position = position_in(ELEMENT_STYLE_COMPLETION_SOURCE, "ba", 2);
    let response = request_json(
        &mut service,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
        30,
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

    assert!(labels.contains(&"background"));
    assert!(!labels.contains(&"routing"));
    assert!(!labels.contains(&"workspace"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_relationship_style_suggests_relationship_style_properties() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("relationship-style-completion.dsl");
    open_document(&mut service, &uri, RELATIONSHIP_STYLE_COMPLETION_SOURCE).await;

    let position = position_in(RELATIONSHIP_STYLE_COMPLETION_SOURCE, "da", 2);
    let response = request_json(
        &mut service,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
        31,
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

    assert!(labels.contains(&"dashed"));
    assert!(!labels.contains(&"background"));
    assert!(!labels.contains(&"workspace"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_style_values_suppresses_property_name_suggestions() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("style-value-completion.dsl");
    open_document(&mut service, &uri, STYLE_VALUE_COMPLETION_SOURCE).await;

    let position = position_in(STYLE_VALUE_COMPLETION_SOURCE, "metadata de", 11);
    let response = request_json(
        &mut service,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
        32,
    )
    .await;

    let items = response["result"]
        .as_array()
        .expect("completion should return an item array");
    assert!(items.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn completion_after_style_block_returns_fixed_vocabulary() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("style-block-end-completion.dsl");
    open_document(&mut service, &uri, STYLE_BLOCK_END_COMPLETION_SOURCE).await;

    let position = position_in(STYLE_BLOCK_END_COMPLETION_SOURCE, "!d", 2);
    let response = request_json(
        &mut service,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
        40,
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

    assert!(labels.contains(&"!docs"));
    assert!(!labels.contains(&"background"));
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

    let customer_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));
    let web_application_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));

    for expectation in [
        DefinitionExpectation {
            needle: "customer -> webApplication",
            byte_offset_within_needle: 1,
            request_id: 10,
            expected_uri: customer_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            needle: "customer -> webApplication",
            byte_offset_within_needle: 13,
            request_id: 11,
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
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source = read_workspace_file(&document_path);
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;

    let people_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));
    let details_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));

    for expectation in [
        DefinitionExpectation {
            needle: "internetBankingSystem customer mainframe email",
            byte_offset_within_needle: 22,
            request_id: 12,
            expected_uri: people_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            needle: "webApplication\n                singlePageApplication",
            byte_offset_within_needle: 0,
            request_id: 13,
            expected_uri: details_uri.as_str(),
            expected_line: 2,
        },
        DefinitionExpectation {
            needle: "developerSinglePageApplicationInstance",
            byte_offset_within_needle: 0,
            request_id: 14,
            expected_uri: document_uri.as_str(),
            expected_line: 31,
        },
        DefinitionExpectation {
            needle: "developerSinglePageApplicationInstance developerWebApplicationInstance developerApiApplicationInstance developerDatabaseInstance",
            byte_offset_within_needle: 39,
            request_id: 15,
            expected_uri: document_uri.as_str(),
            expected_line: 35,
        },
    ] {
        assert_definition_target(&mut service, &document_uri, &document_source, expectation).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn goto_definition_resolves_cross_file_big_bank_dynamic_view_references() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let document_path = workspace_root.join("internet-banking-system.dsl");
    let document_source = read_workspace_file(&document_path);
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;

    let details_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));

    for expectation in [
        DefinitionExpectation {
            needle: "dynamic apiApplication \"SignIn\"",
            byte_offset_within_needle: 8,
            request_id: 70,
            expected_uri: details_uri.as_str(),
            expected_line: 3,
        },
        DefinitionExpectation {
            needle: "singlePageApplication -> signinController \"Submits credentials to\"",
            byte_offset_within_needle: 1,
            request_id: 71,
            expected_uri: details_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            needle: "singlePageApplication -> signinController \"Submits credentials to\"",
            byte_offset_within_needle: 25,
            request_id: 72,
            expected_uri: details_uri.as_str(),
            expected_line: 4,
        },
        DefinitionExpectation {
            needle: "signinController -> securityComponent \"Validates credentials using\"",
            byte_offset_within_needle: 20,
            request_id: 73,
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
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let details_path = workspace_root.join("model/internet-banking-system/details.dsl");
    let details_source = read_workspace_file(&details_path);
    let details_uri = file_uri_from_path(&details_path);
    open_document(&mut service, &details_uri, &details_source).await;

    let docs_position = position_in(&details_source, "!docs docs", 7);
    let docs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": details_uri.as_str() },
            "position": docs_position,
        }),
        33,
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

    let adrs_position = position_in(&details_source, "!adrs adrs", 7);
    let adrs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": details_uri.as_str() },
            "position": adrs_position,
        }),
        34,
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
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let model_path = workspace_root.join("model/people-and-software-systems.dsl");
    let model_source = read_workspace_file(&model_path);
    let model_uri = file_uri_from_path(&model_path);
    open_document(&mut service, &model_uri, &model_source).await;

    let include_position = position_in(
        &model_source,
        "!include \"internet-banking-system/${INTERNET_BANKING_SYSTEM_INCLUDE}\"",
        18,
    );
    let include_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": model_uri.as_str() },
            "position": include_position,
        }),
        35,
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
async fn document_links_resolve_docs_and_adrs_directive_paths() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("big-bank-plc");

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
        36,
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
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let docs_importer_position = position_in(
        &workspace_source,
        "com.example.documentation.CustomDocumentationImporter",
        4,
    );
    let docs_importer_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": docs_importer_position,
        }),
        50,
    )
    .await;
    assert!(docs_importer_response["result"].is_null());

    let adrs_importer_position = position_in(
        &workspace_source,
        "com.example.documentation.CustomDecisionImporter",
        4,
    );
    let adrs_importer_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": adrs_importer_position,
        }),
        51,
    )
    .await;
    assert!(adrs_importer_response["result"].is_null());

    let docs_path_position = position_in(&workspace_source, "!docs docs", 7);
    let docs_path_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": docs_path_position,
        }),
        52,
    )
    .await;
    let docs_uri = file_uri_from_path(
        &fs::canonicalize(temp_workspace.path().join("docs/01-context.md"))
            .expect("docs file should canonicalize"),
    );
    assert_eq!(docs_path_response["result"]["uri"], docs_uri.as_str());

    let adrs_path_position = position_in(&workspace_source, "!adrs decisions", 7);
    let adrs_path_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": adrs_path_position,
        }),
        53,
    )
    .await;
    let adrs_uri = file_uri_from_path(
        &fs::canonicalize(temp_workspace.path().join("decisions/0001-record.md"))
            .expect("adrs file should canonicalize"),
    );
    assert_eq!(adrs_path_response["result"]["uri"], adrs_uri.as_str());
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
async fn document_links_resolve_interpolated_include_paths() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("big-bank-plc");

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
        37,
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
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let docs_position = position_in(&workspace_source, "!docs docs", 7);
    let docs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": docs_position,
        }),
        38,
    )
    .await;
    assert!(docs_response["result"].is_null());

    let adrs_position = position_in(&workspace_source, "!adrs adrs", 7);
    let adrs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": adrs_position,
        }),
        39,
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
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    let docs_position = position_in(&workspace_source, "!docs docs", 7);
    let docs_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": workspace_uri.as_str() },
            "position": docs_position,
        }),
        41,
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
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    for expectation in [
        DefinitionExpectation {
            needle: "containerInstance api",
            byte_offset_within_needle: 18,
            request_id: 12,
            expected_uri: workspace_uri.as_str(),
            expected_line: 3,
        },
        DefinitionExpectation {
            needle: "softwareSystemInstance system",
            byte_offset_within_needle: 23,
            request_id: 13,
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
    let workspace_source = read_workspace_file(&workspace_path);
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, &workspace_source).await;

    for expectation in [
        DefinitionExpectation {
            needle: "primary -> gateway",
            byte_offset_within_needle: 1,
            request_id: 14,
            expected_uri: workspace_uri.as_str(),
            expected_line: 7,
        },
        DefinitionExpectation {
            needle: "gateway -> this",
            byte_offset_within_needle: 1,
            request_id: 15,
            expected_uri: workspace_uri.as_str(),
            expected_line: 8,
        },
        DefinitionExpectation {
            needle: "gateway -> apiInstance",
            byte_offset_within_needle: 11,
            request_id: 16,
            expected_uri: workspace_uri.as_str(),
            expected_line: 9,
        },
    ] {
        assert_definition_target(&mut service, &workspace_uri, &workspace_source, expectation)
            .await;
    }
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

    let web_application_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));
    let mainframe_uri =
        file_uri_from_path(&workspace_root.join("model/people-and-software-systems.dsl"));

    for expectation in [
        DefinitionExpectation {
            needle: "containerInstance webApplication",
            byte_offset_within_needle: 18,
            request_id: 18,
            expected_uri: web_application_uri.as_str(),
            expected_line: 2,
        },
        DefinitionExpectation {
            needle: "softwareSystemInstance mainframe",
            byte_offset_within_needle: 23,
            request_id: 19,
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

#[tokio::test(flavor = "current_thread")]
async fn did_change_republishes_current_document_even_when_diagnostics_are_unchanged() {
    let (mut service, mut socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let uri = file_uri("identifiers-unexpected-tokens-err.dsl");
    open_document(&mut service, &uri, INVALID_SOURCE).await;
    let _ = next_publish_diagnostics_for_uri(&mut socket, uri.as_str()).await;

    let changed_text = format!("{}\n", INVALID_SOURCE);
    change_document(&mut service, &uri, 2, &changed_text).await;

    let notification = next_publish_diagnostics_for_uri(&mut socket, uri.as_str()).await;
    assert_eq!(notification["params"]["version"], 2);
}

#[tokio::test(flavor = "current_thread")]
async fn opening_second_document_does_not_republish_unchanged_diagnostics_for_first_document() {
    let (mut service, mut socket) = new_service();
    let workspace_root = workspace_fixture_path("duplicate-bindings");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let alpha_path = workspace_root.join("alpha.dsl");
    let alpha_uri = file_uri_from_path(&alpha_path);
    open_document(&mut service, &alpha_uri, &read_workspace_file(&alpha_path)).await;
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
    assert_eq!(workspace_notification["params"]["uri"], workspace_uri.as_str());

    let notification = timeout(
        Duration::from_millis(200),
        next_server_notification(&mut socket),
    )
    .await;
    assert!(
        notification.is_err(),
        "opening an unrelated second document should not republish unchanged diagnostics"
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

struct DefinitionExpectation<'a> {
    needle: &'a str,
    byte_offset_within_needle: usize,
    request_id: i64,
    expected_uri: &'a str,
    expected_line: u64,
}

async fn assert_definition_target(
    service: &mut TestService,
    document_uri: &Uri,
    document_source: &str,
    expectation: DefinitionExpectation<'_>,
) {
    let position = position_in(
        document_source,
        expectation.needle,
        expectation.byte_offset_within_needle,
    );
    let response = request_json(
        service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": position,
        }),
        expectation.request_id,
    )
    .await;

    assert_eq!(response["result"]["uri"], expectation.expected_uri);
    assert_eq!(
        response["result"]["range"]["start"]["line"],
        expectation.expected_line
    );
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
