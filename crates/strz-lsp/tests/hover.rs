mod support;

use std::fs;

use indoc::indoc;
use serde_json::json;
use support::{
    annotated_source, file_uri, file_uri_from_path, initialize, initialize_with_workspace_folders,
    initialized, new_service, open_document, request_json, workspace_fixture_path,
};

const SAME_DOCUMENT_SOURCE: &str = r#"workspace {
    model {
        system = softwareSystem "Payments Platform" {
            <CURSOR:api-declaration>api = container "Payments API" "Processes payment requests" "Rust" "Internal, HTTP" {
                technology "Axum"
                tags "Internal, Edge"
                url "https://example.com/api"
            }
            worker = container "Settlement Worker" "Settles payment jobs" "Rust"
        }

        <CURSOR:relationship-declaration>rel = <CURSOR:api-reference>api -> worker "Publishes jobs" "NATS" "Async, Messaging" {
            description "Delivers asynchronous jobs"
            tag "Observed"
            url "https://example.com/rel"
        }
    }
}
"#;
const PLACEHOLDER_RELATIONSHIP_SOURCE: &str = r#"workspace {
    model {
        user = person "User"
        system = softwareSystem "Payments"

        <CURSOR>rel = user -> system "" "HTTPS" "Async, Observed"
    }
}
"#;
const HOVER_METADATA_VIEWS_SOURCE: &str = r#"views {
    container system "Payments" {
        include <CURSOR:api-reference>api
        include <CURSOR:relationship-reference>rel
        autoLayout
    }
}
"#;
const HIERARCHICAL_CONTEXT_VIEWS_SOURCE: &str = r#"views {
    component system.api "API components" {
        include <CURSOR:system-reference>system.<CURSOR:api-reference>api.<CURSOR:worker-reference>worker
        autoLayout
    }
}
"#;
const DUPLICATE_BINDINGS_WORKSPACE_SOURCE: &str = r#"workspace {
    !include "alpha.dsl"
    !include "beta.dsl"

    model {
        user = person "User"
        user -> <CURSOR:ambiguous-api>api "Calls"
    }
}
"#;
const DEPLOYMENT_INSTANCE_HOVER_SOURCE: &str = r#"workspace {
    model {
        system = softwareSystem "Payments"

        deploymentEnvironment "Live" {
            edge = deploymentNode "Edge" {
                <CURSOR:instance-declaration>canary = softwareSystemInstance system blue "Canary" {
                    tag "Observed"
                    url "https://example.com/canary"
                }
            }
        }
    }
}
"#;
const HIERARCHICAL_SELECTOR_HOVER_SOURCE: &str = r#"workspace {
    model {
        !identifiers hierarchical

        system = softwareSystem "Payments Platform" {
            <CURSOR:api-declaration>api = container "Payments API" "Processes payment requests" "Rust" "Internal, HTTP" {
                technology "Axum"
                tags "Internal, Edge"
                url "https://example.com/api"
            }
            worker = container "Settlement Worker" "Settles payment jobs" "Rust"

            !element <CURSOR:selector-target>api {
                worker -> <CURSOR:this-reference>this "Targets selector"
            }

            worker -> <CURSOR:dotted-reference>system.api "Uses"
        }
    }
}
"#;
const SELECTOR_SEGMENT_HOVER_SOURCE: &str = r#"workspace {
    !identifiers flat

    model {
        !identifiers hierarchical

        system = softwareSystem "System" {
            api = container "API" {
                worker = component "Worker"
            }
        }

        !element <CURSOR:selector-system>system.<CURSOR:selector-api>api.<CURSOR:selector-worker>worker {
            properties {
                "team" "Core"
            }
        }
    }
}
"#;

const API_HOVER: &str = indoc! {"
    **Container** `api`
    Payments API

    Processes payment requests

    **Technology:** Axum  
    **Tags:** Internal, HTTP, Edge  
    **URL:** <https://example.com/api>"};
const API_HOVER_WITH_WORKSPACE_CONTEXT: &str = indoc! {"
    **Container** `api`
    Payments API

    Processes payment requests

    **Technology:** Axum  
    **Tags:** Internal, HTTP, Edge  
    **URL:** <https://example.com/api>

    **Canonical key:** `api`  
    **Parent chain:** Software System `system`  
    **Declaration path:** `model.dsl`"};
const RELATIONSHIP_HOVER: &str = indoc! {"
    **Relationship** `rel`
    Publishes jobs

    Delivers asynchronous jobs

    **Technology:** NATS  
    **Tags:** Async, Messaging, Observed  
    **URL:** <https://example.com/rel>"};
const RELATIONSHIP_HOVER_WITH_WORKSPACE_CONTEXT: &str = indoc! {"
    **Relationship** `rel`
    Publishes jobs

    Delivers asynchronous jobs

    **Technology:** NATS  
    **Tags:** Async, Messaging, Observed  
    **URL:** <https://example.com/rel>

    **Canonical key:** `rel`  
    **Declaration path:** `model.dsl`  
    **Endpoints:** Container `api` → Container `worker`"};
const PLACEHOLDER_RELATIONSHIP_HOVER: &str = indoc! {"
    **Relationship** `rel`

    **Technology:** HTTPS  
    **Tags:** Async, Observed"};
const DEPLOYMENT_INSTANCE_HOVER: &str = indoc! {"
    **Software System Instance** `canary`

    **Tags:** Canary, Observed  
    **URL:** <https://example.com/canary>"};
const SYSTEM_HOVER: &str = indoc! {"
    **Software System** `system`
    System"};
const WORKER_HOVER: &str = indoc! {"
    **Component** `worker`
    Worker"};
const WORKER_HIERARCHICAL_HOVER: &str = indoc! {"
    **Component** `worker`
    Worker

    **Canonical key:** `system.api.worker`  
    **Parent chain:** Software System `system` → Container `api`  
    **Declaration path:** `model.dsl`"};

#[tokio::test(flavor = "current_thread")]
async fn hover_returns_markdown_for_same_document_declarations() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(SAME_DOCUMENT_SOURCE);
    let uri = file_uri("hover-same-document.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let hover = request_hover(&mut service, &uri, source.position("api-declaration")).await;

    assert_hover_markdown(&hover, API_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_returns_markdown_for_same_document_references() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(SAME_DOCUMENT_SOURCE);
    let uri = file_uri("hover-same-document.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let api_hover = request_hover(&mut service, &uri, source.position("api-reference")).await;
    assert_hover_markdown(&api_hover, API_HOVER);

    let relationship_hover = request_hover(
        &mut service,
        &uri,
        source.position("relationship-declaration"),
    )
    .await;
    assert_hover_markdown(&relationship_hover, RELATIONSHIP_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_resolves_hierarchical_selector_and_dotted_reference_sites() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(HIERARCHICAL_SELECTOR_HOVER_SOURCE);
    let uri = file_uri("hover-hierarchical-selector.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let selector_hover =
        request_hover(&mut service, &uri, source.position("selector-target")).await;
    assert_hover_markdown(&selector_hover, API_HOVER);

    let dotted_hover = request_hover(&mut service, &uri, source.position("dotted-reference")).await;
    assert_hover_markdown(&dotted_hover, API_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_resolves_each_selector_segment_to_its_own_binding() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(SELECTOR_SEGMENT_HOVER_SOURCE);
    let uri = file_uri("hover-selector-segments.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let system_hover = request_hover(&mut service, &uri, source.position("selector-system")).await;
    assert_hover_markdown(&system_hover, SYSTEM_HOVER);

    let api_hover = request_hover(&mut service, &uri, source.position("selector-api")).await;
    assert_hover_markdown(&api_hover, "**Container** `api`\nAPI");

    let worker_hover = request_hover(&mut service, &uri, source.position("selector-worker")).await;
    assert_hover_markdown(&worker_hover, WORKER_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_includes_inline_deployment_instance_tags() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(DEPLOYMENT_INSTANCE_HOVER_SOURCE);
    let uri = file_uri("hover-deployment-instance.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let hover = request_hover(&mut service, &uri, source.position("instance-declaration")).await;

    assert_hover_markdown(&hover, DEPLOYMENT_INSTANCE_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_preserves_empty_relationship_placeholder_slots() {
    let (mut service, _socket) = new_service();

    initialize(&mut service).await;
    initialized(&mut service).await;

    let source = annotated_source(PLACEHOLDER_RELATIONSHIP_SOURCE);
    let uri = file_uri("hover-placeholder-relationship.dsl");
    open_document(&mut service, &uri, source.source()).await;

    let hover = request_hover(&mut service, &uri, source.only_position()).await;

    assert_hover_markdown(&hover, PLACEHOLDER_RELATIONSHIP_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_resolves_cross_file_symbols_through_workspace_indexes() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("hover-metadata");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let views_path = workspace_root.join("views.dsl");
    let views_source = annotated_source(HOVER_METADATA_VIEWS_SOURCE);
    assert_fixture_source(&views_path, views_source.source());
    let views_uri = file_uri_from_path(&views_path);
    open_document(&mut service, &views_uri, views_source.source()).await;

    let api_hover = request_hover(
        &mut service,
        &views_uri,
        views_source.position("api-reference"),
    )
    .await;
    assert_hover_markdown(&api_hover, API_HOVER_WITH_WORKSPACE_CONTEXT);

    let relationship_hover = request_hover(
        &mut service,
        &views_uri,
        views_source.position("relationship-reference"),
    )
    .await;
    assert_hover_markdown(
        &relationship_hover,
        RELATIONSHIP_HOVER_WITH_WORKSPACE_CONTEXT,
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hover_displays_hierarchical_canonical_context() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("hover-hierarchical");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let views_path = workspace_root.join("views.dsl");
    let views_source = annotated_source(HIERARCHICAL_CONTEXT_VIEWS_SOURCE);
    assert_fixture_source(&views_path, views_source.source());
    let views_uri = file_uri_from_path(&views_path);
    open_document(&mut service, &views_uri, views_source.source()).await;

    let system_hover = request_hover(
        &mut service,
        &views_uri,
        views_source.position("system-reference"),
    )
    .await;
    assert_hover_markdown(&system_hover, WORKER_HIERARCHICAL_HOVER);

    let api_hover = request_hover(
        &mut service,
        &views_uri,
        views_source.position("api-reference"),
    )
    .await;
    assert_hover_markdown(&api_hover, WORKER_HIERARCHICAL_HOVER);

    let worker_hover = request_hover(
        &mut service,
        &views_uri,
        views_source.position("worker-reference"),
    )
    .await;

    assert_hover_markdown(&worker_hover, WORKER_HIERARCHICAL_HOVER);
}

#[tokio::test(flavor = "current_thread")]
async fn hover_resolves_relationship_endpoint_symbols_declared_via_include() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("big-bank-plc");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("internet-banking-system.dsl");
    let annotated_source = annotated_source(
        &read_workspace_file(&workspace_path).replacen(
            "customer -> webApplication",
            "customer -> <CURSOR:web-relationship>webApplication",
            1,
        ),
    );
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, annotated_source.source()).await;

    let hover = request_hover(
        &mut service,
        &workspace_uri,
        annotated_source.position("web-relationship"),
    )
    .await;

    assert!(
        !hover["result"].is_null(),
        "relationship endpoint hover should resolve across !include declarations"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hover_returns_no_result_for_ambiguous_workspace_references() {
    let (mut service, _socket) = new_service();
    let workspace_root = workspace_fixture_path("duplicate-bindings");

    initialize_with_workspace_folders(&mut service, &[file_uri_from_path(&workspace_root)]).await;
    initialized(&mut service).await;

    let workspace_path = workspace_root.join("workspace.dsl");
    let workspace_source = annotated_source(DUPLICATE_BINDINGS_WORKSPACE_SOURCE);
    assert_fixture_source(&workspace_path, workspace_source.source());
    let workspace_uri = file_uri_from_path(&workspace_path);
    open_document(&mut service, &workspace_uri, workspace_source.source()).await;

    let hover = request_hover(
        &mut service,
        &workspace_uri,
        workspace_source.position("ambiguous-api"),
    )
    .await;

    assert!(hover["result"].is_null());
}

async fn request_hover(
    service: &mut support::TestService,
    uri: &tower_lsp_server::ls_types::Uri,
    position: tower_lsp_server::ls_types::Position,
) -> serde_json::Value {
    request_json(
        service,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        }),
    )
    .await
}

fn assert_hover_markdown(response: &serde_json::Value, expected: &str) {
    assert_eq!(response["result"]["contents"]["kind"], "markdown");
    assert_eq!(
        response["result"]["contents"]["value"]
            .as_str()
            .expect("hover markdown should be returned as a string"),
        expected
    );
}

fn assert_fixture_source(path: &std::path::Path, annotated_source: &str) {
    assert_eq!(
        read_workspace_file(path),
        annotated_source,
        "annotated source should match workspace fixture `{}`",
        path.display()
    );
}

fn read_workspace_file(path: &std::path::Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "workspace fixture `{}` should be readable: {error}",
            path.display()
        )
    })
}
