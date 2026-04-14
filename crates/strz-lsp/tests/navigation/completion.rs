use std::path::Path;

use indoc::{formatdoc, indoc};
use serde_json::json;
use tower_lsp_server::ls_types::{Position, Uri};

use crate::support::{
    AnnotatedSource, TempWorkspace, TestService, annotated_source, change_document, file_uri,
    file_uri_from_path, initialize, initialize_with_workspace_folders, initialized, new_service,
    next_publish_diagnostics_for_uri, open_document, read_workspace_file, request_json,
    workspace_fixture_path,
};

// Keep one representative deployment workspace inline so each completion test
// varies only the relationship line under review, not the surrounding model.
fn deployment_completion_source(relation_line: &str) -> AnnotatedSource {
    deployment_completion_source_with_preamble("", relation_line)
}

fn deployment_completion_source_with_preamble(
    preamble: &str,
    relation_line: &str,
) -> AnnotatedSource {
    annotated_source(&formatdoc! {r#"
        workspace {{
          model {{
            {preamble}
            system = softwareSystem "System" {{
              api = container "API"
            }}

            live = deploymentEnvironment "Live" {{
              primary = deploymentNode "Primary" {{
                gateway = infrastructureNode "Gateway"
                systemInstance = softwareSystemInstance system
                apiInstance = containerInstance api
              }}
              secondary = deploymentNode "Secondary" {{
                cdn = infrastructureNode "CDN"
                secondaryApiInstance = containerInstance api
              }}

              {relation_line}
            }}
          }}
        }}
    "#})
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
async fn completion_inside_deployment_relationship_source_suggests_deployment_identifiers() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source("ga<CURSOR> -> secondaryApiInstance");
    let uri = file_uri("deployment-relationship-source-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert_eq!(labels, vec!["gateway"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_blank_deployment_relationship_source_suggests_deployment_identifiers() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source("<CURSOR>-> secondaryApiInstance");
    let uri = file_uri("blank-deployment-relationship-source-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let mut labels = completion_labels(&mut service, &uri, &source).await;
    labels.sort_unstable();

    assert_eq!(
        labels,
        vec![
            "apiInstance",
            "cdn",
            "gateway",
            "primary",
            "secondary",
            "secondaryApiInstance",
            "systemInstance",
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_fresh_deployment_relationship_source_uses_deployment_bindings() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source("pri<CURSOR>");
    let uri = file_uri("fresh-deployment-relationship-source-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert_eq!(labels, vec!["primary"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_deployment_node_relationship_destination_suggests_only_deployment_nodes()
{
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source("primary -> <CURSOR>");
    let uri = file_uri("deployment-node-destination-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let mut labels = completion_labels(&mut service, &uri, &source).await;
    labels.sort_unstable();

    assert_eq!(labels, vec!["primary", "secondary"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_infrastructure_node_relationship_destination_uses_deployment_matrix() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source("gateway -> <CURSOR>");
    let uri = file_uri("infrastructure-node-destination-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let mut labels = completion_labels(&mut service, &uri, &source).await;
    labels.sort_unstable();

    assert_eq!(
        labels,
        vec![
            "apiInstance",
            "cdn",
            "gateway",
            "primary",
            "secondary",
            "secondaryApiInstance",
            "systemInstance",
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_workspace_backed_deployment_relationship_uses_deployment_bindings() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("deployment-navigation");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = annotated_source(&read_workspace_file(&workspace_path).replacen(
        "gateway -> secondaryApiInstance \"Routes traffic\"",
        "gateway -> sec<CURSOR>",
        1,
    ));
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let mut labels = completion_labels(&mut service, &workspace_uri, &workspace_source).await;
    labels.sort_unstable();

    assert_eq!(labels, vec!["secondary", "secondaryApiInstance"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_software_system_instance_relationship_destination_suggests_only_infrastructure_nodes()
 {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source("systemInstance -> <CURSOR>");
    let uri = file_uri("software-system-instance-destination-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let mut labels = completion_labels(&mut service, &uri, &source).await;
    labels.sort_unstable();

    assert_eq!(labels, vec!["cdn", "gateway"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_container_instance_relationship_destination_suggests_only_infrastructure_nodes()
 {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source("apiInstance -> <CURSOR>");
    let uri = file_uri("container-instance-destination-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let mut labels = completion_labels(&mut service, &uri, &source).await;
    labels.sort_unstable();

    assert_eq!(labels, vec!["cdn", "gateway"]);
}

#[tokio::test(flavor = "current_thread")]
async fn completion_inside_deployment_relationship_destination_suppresses_hierarchical_mode() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = deployment_completion_source_with_preamble(
        "!identifiers hierarchical",
        "primary -> <CURSOR>",
    );
    let uri = file_uri("deployment-relationship-hierarchical-completion.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let labels = completion_labels(&mut service, &uri, &source).await;

    assert!(labels.is_empty());
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
        "workspace {\n  !include relationships.dsl\n}\n",
        &[],
        &[
            (
                Path::new("model.dsl"),
                "customer = person \"Customer\"\nwebApplication = softwareSystem \"Web Application\"\n",
            ),
            (
                Path::new("relationships.dsl"),
                "model {\n  !include model.dsl\n  customer -> webApplication \"Uses\"\n\n  deploymentEnvironment \"Development\" {\n  }\n}\n",
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
