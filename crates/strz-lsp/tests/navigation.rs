mod support;

use std::{fs, path::Path};

use indoc::indoc;
use serde_json::json;
use support::{
    AnnotatedSource, TempWorkspace, TestService, annotated_source, change_document, close_document,
    file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders, initialized,
    new_service, next_publish_diagnostics_for_uri, open_document, position_in, read_workspace_file,
    request_json, workspace_fixture_path,
};
use tempfile::TempDir;
use tower_lsp_server::ls_types::{Position, Uri};

const DIRECT_REFERENCES_SOURCE: &str =
    include_str!("fixtures/relationships/named-relationships-ok.dsl");

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

#[tokio::test(flavor = "current_thread")]
async fn completion_returns_directive_keywords_for_prefixes() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r"
        workspace {
          !i<CURSOR>
        }
    "});
    let uri = file_uri("completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.iter().any(|label| label == "!include"));
    assert!(labels.iter().any(|label| label == "!identifiers"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_element_style_suggests_element_style_properties() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                ba<CURSOR>
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.iter().any(|label| label == "background"));
    assert!(!labels.iter().any(|label| label == "routing"));
    assert!(!labels.iter().any(|label| label == "workspace"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_relationship_style_suggests_relationship_style_properties() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              relationship "Uses" {
                da<CURSOR>
              }
            }
          }
        }
    "#});
    let uri = file_uri("relationship-style-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.iter().any(|label| label == "dashed"));
    assert!(!labels.iter().any(|label| label == "background"));
    assert!(!labels.iter().any(|label| label == "workspace"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_element_style_color_values_suggests_named_colors() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                background dark<CURSOR:background>
                color dark<CURSOR:color>
                colour dark<CURSOR:colour>
                stroke dark<CURSOR:stroke>
                background #ff<CURSOR:hex>
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-color-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    for marker in ["background", "color", "colour", "stroke"] {
        let labels =
            completion_labels_at_position(&mut service, &uri, source.position(marker)).await;
        assert!(
            labels.iter().any(|label| label == "darkseagreen"),
            "expected named colours for `{marker}`, got {labels:?}"
        );
        assert!(
            !labels.iter().any(|label| label == "RoundedBox"),
            "expected colour-only completions for `{marker}`, got {labels:?}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_hex_color_prefix_suppresses_named_color_suggestions() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                background dark<CURSOR:background>
                color dark<CURSOR:color>
                colour dark<CURSOR:colour>
                stroke dark<CURSOR:stroke>
                background #ff<CURSOR:hex>
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-hex-color-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels_at_position(&mut service, &uri, source.position("hex")).await;
    assert!(labels.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_element_style_boolean_values_suggests_true_false() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                metadata t<CURSOR:metadata>
                description f<CURSOR:description>
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-boolean-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let metadata_labels =
        completion_labels_at_position(&mut service, &uri, source.position("metadata")).await;
    assert_eq!(metadata_labels, vec!["true"]);

    let description_labels =
        completion_labels_at_position(&mut service, &uri, source.position("description")).await;
    assert_eq!(description_labels, vec!["false"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_element_style_border_values_suggests_matching_values() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                border d<CURSOR>
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-border-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;
    assert!(labels.iter().any(|label| label == "Dashed"));
    assert!(labels.iter().any(|label| label == "Dotted"));
    assert!(!labels.iter().any(|label| label == "Solid"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_quoted_element_style_shape_values_suggests_matching_values() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                shape "rou<CURSOR>"
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-quoted-shape-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;
    assert_eq!(labels, vec!["RoundedBox"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_style_properties_block_values_returns_no_items() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                properties {
                  background dark<CURSOR:background>
                  shape "rou<CURSOR:shape>"
                }
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-properties-block-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    for marker in ["background", "shape"] {
        let labels =
            completion_labels_at_position(&mut service, &uri, source.position(marker)).await;
        assert!(
            labels.is_empty(),
            "expected no style-value completions inside properties block for `{marker}`, got {labels:?}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_style_comments_returns_no_items() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                background # dark<CURSOR>
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-comment-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;
    assert!(labels.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn completion_does_not_recover_values_across_lines() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                background
                  dark<CURSOR>
              }
            }
          }
        }
    "#});
    let uri = file_uri("element-style-cross-line-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;
    assert!(
        labels.is_empty(),
        "expected cross-line value recovery to stay suppressed, got {labels:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_relationship_style_values_still_returns_no_items() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              relationship "Uses" {
                metadata de<CURSOR>
              }
            }
          }
        }
    "#});
    let uri = file_uri("style-value-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;
    assert!(labels.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn completion_after_style_block_returns_fixed_vocabulary() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          views {
            styles {
              element "Person" {
                background #ffffff
              }
              !d<CURSOR>
            }
          }
        }
    "#});
    let uri = file_uri("style-block-end-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.iter().any(|label| label == "!docs"));
    assert!(!labels.iter().any(|label| label == "background"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_relationship_source_suggests_core_identifiers() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          model {
            user = person "User"
            system = softwareSystem "System" {
              api = container "API" {
                worker = component "Worker"
              }
            }

            <CURSOR>-> system "Uses"
          }
        }
    "#});
    let uri = file_uri("relationship-source-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.iter().any(|label| label == "user"));
    assert!(labels.iter().any(|label| label == "system"));
    assert!(labels.iter().any(|label| label == "api"));
    assert!(labels.iter().any(|label| label == "worker"));
    assert!(!labels.iter().any(|label| label == "workspace"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_fresh_relationship_source_suggests_core_identifiers() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          model {
            user = person "User"
            system = softwareSystem "System" {
              api = container "API" {
                worker = component "Worker"
              }
            }

            u<CURSOR>
          }
        }
    "#});
    let uri = file_uri("fresh-relationship-source-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.iter().any(|label| label == "user"));
    assert!(!labels.iter().any(|label| label == "workspace"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_relationship_destination_suggests_core_identifiers() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          model {
            user = person "User"
            system = softwareSystem "System" {
              api = container "API" {
                worker = component "Worker"
              }
            }

            user -> <CURSOR>
          }
        }
    "#});
    let uri = file_uri("relationship-destination-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.iter().any(|label| label == "user"));
    assert!(labels.iter().any(|label| label == "system"));
    assert!(labels.iter().any(|label| label == "api"));
    assert!(labels.iter().any(|label| label == "worker"));
    assert!(!labels.iter().any(|label| label == "workspace"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_unterminated_relationship_string_returns_no_items() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          model {
            user = person "User"
            system = softwareSystem "System"

            user -> system "<CURSOR>c
          }
        }
    "#});
    let uri = file_uri("open-relationship-string-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(
        labels.is_empty(),
        "expected no completion labels inside an open relationship string, got {labels:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_unterminated_workspace_string_returns_no_items() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace "<CURSOR>c
    "#});
    let uri = file_uri("open-workspace-string-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(
        labels.is_empty(),
        "expected no completion labels inside an open workspace string, got {labels:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_relationship_destination_uses_workspace_symbols_across_files() {
    let temp_workspace = TempWorkspace::new(
        "relationship-completion-cross-file",
        "workspace {\n  !include shared/model.dsl\n  !include shared/relationships.dsl\n}\n",
        &[Path::new("shared")],
        &[
            (
                Path::new("shared/model.dsl"),
                "model {\n  user = person \"User\"\n  system = softwareSystem \"System\" {\n    api = container \"API\"\n  }\n}\n",
            ),
            (
                Path::new("shared/relationships.dsl"),
                "model {\n  user -> \n}\n",
            ),
        ],
    );
    let (mut service, _socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let relationships_path = temp_workspace.path().join("shared/relationships.dsl");
    let relationships_source = annotated_source(
        &read_workspace_file(&relationships_path).replacen("user -> ", "user -> <CURSOR>", 1),
    );
    let relationships_uri = file_uri_from_path(&relationships_path);
    open_document(
        &mut service,
        &relationships_uri,
        relationships_source.source(),
    )
    .await;

    let labels = completion_labels(&mut service, &relationships_uri, &relationships_source).await;

    assert!(labels.iter().any(|label| label == "user"));
    assert!(labels.iter().any(|label| label == "system"));
    assert!(labels.iter().any(|label| label == "api"));
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_fresh_relationship_source_before_deployment_environment_suggests_workspace_identifiers()
 {
    let temp_workspace = TempWorkspace::new(
        "relationship-completion-before-deployment-environment",
        "workspace {\n  !include model.dsl\n  !include relationships.dsl\n}\n",
        &[],
        &[
            (
                Path::new("model.dsl"),
                "model {\n  customer = person \"Customer\"\n  webApplication = softwareSystem \"Web Application\"\n}\n",
            ),
            (
                Path::new("relationships.dsl"),
                "model {\n  customer -> webApplication \"Uses\"\n\n  deploymentEnvironment \"Development\" {\n  }\n}\n",
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
        "\n\n  deploymentEnvironment",
        "\n\n  cust<CURSOR>\n  deploymentEnvironment",
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
        "partial relationship source before deploymentEnvironment should not publish cascaded syntax diagnostics: {diagnostics:?}"
    );

    let labels = completion_labels(&mut service, &document_uri, &fresh_source).await;

    assert!(
        labels.iter().any(|label| label == "customer"),
        "expected `customer` in labels, got {labels:?}"
    );
}

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
async fn completion_inside_relationship_destination_suppresses_hierarchical_mode() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(indoc! {r#"
        workspace {
          model {
            !identifiers hierarchical

            system = softwareSystem "System" {
              api = container "API"
            }

            system -> <CURSOR>
          }
        }
    "#});
    let uri = file_uri("relationship-hierarchical-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_multi_instance_relationship_fragment_returns_no_result() {
    let temp_workspace = TempWorkspace::new(
        "relationship-completion-multi-instance",
        "workspace {\n  !include shared/model-alpha.dsl\n  !include shared/relationships.dsl\n}\n",
        &[Path::new("shared")],
        &[
            (
                Path::new("beta.dsl"),
                "workspace {\n  !include shared/model-beta.dsl\n  !include shared/relationships.dsl\n}\n",
            ),
            (
                Path::new("shared/model-alpha.dsl"),
                "model {\n  user = person \"User\"\n  systemAlpha = softwareSystem \"Alpha\"\n}\n",
            ),
            (
                Path::new("shared/model-beta.dsl"),
                "model {\n  user = person \"User\"\n  systemBeta = softwareSystem \"Beta\"\n}\n",
            ),
            (
                Path::new("shared/relationships.dsl"),
                "model {\n  user -> \n}\n",
            ),
        ],
    );
    let (mut service, _socket) = new_service();

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(temp_workspace.path())])
        .await;
    initialized(&mut service).await;

    let relationships_path = temp_workspace.path().join("shared/relationships.dsl");
    let relationships_source = annotated_source(
        &read_workspace_file(&relationships_path).replacen("user -> ", "user -> <CURSOR>", 1),
    );
    let relationships_uri = file_uri_from_path(&relationships_path);
    open_document(
        &mut service,
        &relationships_uri,
        relationships_source.source(),
    )
    .await;

    let labels = completion_labels(&mut service, &relationships_uri, &relationships_source).await;

    assert!(labels.is_empty());
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
            expected_uri: customer_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            needle: "customer -> webApplication",
            byte_offset_within_needle: 13,
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
            expected_uri: people_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            needle: "webApplication\n                singlePageApplication",
            byte_offset_within_needle: 0,
            expected_uri: details_uri.as_str(),
            expected_line: 2,
        },
    ] {
        assert_definition_target(&mut service, &document_uri, &document_source, expectation).await;
    }

    let animation_needle = "developerSinglePageApplicationInstance developerWebApplicationInstance developerApiApplicationInstance developerDatabaseInstance";
    let animation_position = position_in(&document_source, animation_needle, 0);
    let animation_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": animation_position,
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

    let animation_web_position = position_in(&document_source, animation_needle, 39);
    let animation_web_response = request_json(
        &mut service,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": document_uri.as_str() },
            "position": animation_web_position,
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
    let document_source = read_workspace_file(&document_path);
    let document_uri = file_uri_from_path(&document_path);
    open_document(&mut service, &document_uri, &document_source).await;

    let details_uri =
        file_uri_from_path(&workspace_root.join("model/internet-banking-system/details.dsl"));

    for expectation in [
        DefinitionExpectation {
            needle: "dynamic apiApplication \"SignIn\"",
            byte_offset_within_needle: 8,
            expected_uri: details_uri.as_str(),
            expected_line: 3,
        },
        DefinitionExpectation {
            needle: "singlePageApplication -> signinController \"Submits credentials to\"",
            byte_offset_within_needle: 1,
            expected_uri: details_uri.as_str(),
            expected_line: 0,
        },
        DefinitionExpectation {
            needle: "singlePageApplication -> signinController \"Submits credentials to\"",
            byte_offset_within_needle: 25,
            expected_uri: details_uri.as_str(),
            expected_line: 4,
        },
        DefinitionExpectation {
            needle: "signinController -> securityComponent \"Validates credentials using\"",
            byte_offset_within_needle: 20,
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
            expected_uri: workspace_uri.as_str(),
            expected_line: 3,
        },
        DefinitionExpectation {
            needle: "softwareSystemInstance system",
            byte_offset_within_needle: 23,
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
            expected_uri: workspace_uri.as_str(),
            expected_line: 7,
        },
        DefinitionExpectation {
            needle: "gateway -> this",
            byte_offset_within_needle: 1,
            expected_uri: workspace_uri.as_str(),
            expected_line: 8,
        },
        DefinitionExpectation {
            needle: "gateway -> apiInstance",
            byte_offset_within_needle: 11,
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
    )
    .await;

    assert!(response["result"].is_null());
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
            expected_uri: web_application_uri.as_str(),
            expected_line: 2,
        },
        DefinitionExpectation {
            needle: "softwareSystemInstance mainframe",
            byte_offset_within_needle: 23,
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

fn copied_workspace_fixture(name: &str) -> TempDir {
    let source_root = workspace_fixture_path(name);
    let temp_dir = tempfile::Builder::new()
        .prefix(name)
        .tempdir()
        .expect("temp fixture workspace should create");
    copy_workspace_fixture_dir(&source_root, temp_dir.path());
    temp_dir
}

fn copy_workspace_fixture_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap_or_else(|error| {
        panic!(
            "failed to create temp fixture directory `{}`: {error}",
            destination.display()
        )
    });

    for entry in fs::read_dir(source).unwrap_or_else(|error| {
        panic!(
            "failed to read workspace fixture directory `{}`: {error}",
            source.display()
        )
    }) {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to read entry in workspace fixture directory `{}`: {error}",
                source.display()
            )
        });
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().unwrap_or_else(|error| {
            panic!(
                "failed to read file type for workspace fixture entry `{}`: {error}",
                source_path.display()
            )
        });

        if file_type.is_dir() {
            copy_workspace_fixture_dir(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap_or_else(|error| {
                panic!(
                    "failed to copy workspace fixture file `{}` to `{}`: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            });
        }
    }
}

struct DefinitionExpectation<'a> {
    needle: &'a str,
    byte_offset_within_needle: usize,
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
    )
    .await;

    assert_eq!(response["result"]["uri"], expectation.expected_uri);
    assert_eq!(
        response["result"]["range"]["start"]["line"],
        expectation.expected_line
    );
}

async fn completion_labels(
    service: &mut TestService,
    uri: &Uri,
    source: &AnnotatedSource,
) -> Vec<String> {
    completion_labels_at_position(service, uri, source.only_position()).await
}

async fn completion_labels_at_position(
    service: &mut TestService,
    uri: &Uri,
    position: Position,
) -> Vec<String> {
    let response = request_json(
        service,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
    )
    .await;

    response["result"]
        .as_array()
        .expect("completion should return an item array")
        .iter()
        .map(|item| {
            item["label"]
                .as_str()
                .expect("completion label should be a string")
                .to_owned()
        })
        .collect()
}
