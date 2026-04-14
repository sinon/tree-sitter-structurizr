use indoc::indoc;
use strz_analysis::{
    DocumentAnalyzer, DocumentInput, DynamicViewStepFact, ImageSourceKind, ImageSourceMode,
    ResourceDirectiveKind, ViewKind, WorkspaceScope, WorkspaceSectionKind,
};

fn analyze(source: &str) -> strz_analysis::DocumentSnapshot {
    let mut analyzer = DocumentAnalyzer::new();
    analyzer.analyze(DocumentInput::new("workspace.dsl", source))
}

fn workspace_view_and_resource_snapshot() -> strz_analysis::DocumentSnapshot {
    analyze(indoc! {r#"
        workspace {
            !docs "docs"

            model {
                system = softwareSystem "System" {
                    !docs docs
                    !adrs decisions madr
                    api = container "API"
                }

                !element system.api {
                    infrastructureNode "Helper"
                }

                system -> api "Uses" "HTTPS"
            }

            views {
                properties {
                    "plantuml.url" "https://example.com/plantuml"
                }

                container system "container-view" {
                    include api
                    animation {
                        api
                    }
                    autoLayout lr 300 200
                }

                filtered "container-view" include "Element,Relationship" "filtered-view"

                dynamic system "dynamic-view" {
                    1: system -> api "Uses" "HTTPS"
                }

                image * "image-view" {
                    properties {
                        "mermaid.url" "https://example.com/mermaid"
                    }
                    plantuml "diagram.puml"
                }
            }

            configuration {
                scope softwareSystem
            }
        }
    "#})
}

fn assert_workspace_sections_and_scope(snapshot: &strz_analysis::DocumentSnapshot) {
    assert_eq!(
        snapshot
            .workspace_sections()
            .iter()
            .map(|section| section.kind)
            .collect::<Vec<_>>(),
        vec![
            WorkspaceSectionKind::Model,
            WorkspaceSectionKind::Views,
            WorkspaceSectionKind::Configuration,
        ]
    );

    let scope = snapshot
        .configuration_scopes()
        .first()
        .expect("configuration scope should exist");
    assert_eq!(scope.scope, WorkspaceScope::SoftwareSystem);
    assert_eq!(scope.value.normalized_text, "softwareSystem");
}

fn assert_resource_selector_and_property_facts(snapshot: &strz_analysis::DocumentSnapshot) {
    assert_eq!(
        snapshot
            .resource_directives()
            .iter()
            .map(|directive| (
                directive.kind,
                directive.path.normalized_text.as_str(),
                directive
                    .importer
                    .as_ref()
                    .map(|value| value.normalized_text.as_str()),
                directive.container_node_kind.as_str(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (ResourceDirectiveKind::Docs, "docs", None, "workspace_block",),
            (
                ResourceDirectiveKind::Docs,
                "docs",
                None,
                "software_system_block",
            ),
            (
                ResourceDirectiveKind::Adrs,
                "decisions",
                Some("madr"),
                "software_system_block",
            ),
        ]
    );

    assert_eq!(
        snapshot
            .property_facts()
            .iter()
            .map(|property| (
                property.name.normalized_text.as_str(),
                property.value.normalized_text.as_str(),
                property.container_node_kind.as_str(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "plantuml.url",
                "https://example.com/plantuml",
                "views_block",
            ),
            (
                "mermaid.url",
                "https://example.com/mermaid",
                "image_view_block",
            ),
        ]
    );

    let selector = snapshot
        .element_directives()
        .first()
        .expect("element directive should exist");
    assert_eq!(selector.target.normalized_text, "system.api");
    assert_eq!(selector.container_node_kind, "model_block");
}

fn find_view<'a>(
    snapshot: &'a strz_analysis::DocumentSnapshot,
    key: &str,
) -> &'a strz_analysis::ViewFact {
    snapshot
        .view_facts()
        .iter()
        .find(|view| {
            view.key
                .as_ref()
                .is_some_and(|view_key| view_key.normalized_text == key)
        })
        .unwrap_or_else(|| panic!("view `{key}` should exist"))
}

fn assert_container_and_filtered_views(snapshot: &strz_analysis::DocumentSnapshot) {
    let container_view = find_view(snapshot, "container-view");
    assert_eq!(container_view.kind, ViewKind::Container);
    assert_eq!(
        container_view
            .scope
            .as_ref()
            .expect("container view scope should exist")
            .normalized_text,
        "system"
    );
    let auto_layout = container_view
        .auto_layout
        .as_ref()
        .expect("container view autolayout should exist");
    assert_eq!(auto_layout.direction.as_deref(), Some("lr"));
    assert_eq!(auto_layout.rank_separation.as_deref(), Some("300"));
    assert_eq!(auto_layout.node_separation.as_deref(), Some("200"));
    assert_eq!(
        container_view
            .include_values
            .iter()
            .map(|value| value.normalized_text.as_str())
            .collect::<Vec<_>>(),
        vec!["api"]
    );
    assert_eq!(
        container_view
            .animation_values
            .iter()
            .map(|value| value.normalized_text.as_str())
            .collect::<Vec<_>>(),
        vec!["api"]
    );

    let filtered_view = find_view(snapshot, "filtered-view");
    assert_eq!(filtered_view.kind, ViewKind::Filtered);
    assert_eq!(
        filtered_view
            .base_key
            .as_ref()
            .expect("filtered view base key should exist")
            .normalized_text,
        "container-view"
    );
    assert_eq!(filtered_view.filter_mode.as_deref(), Some("include"));
    assert_eq!(
        filtered_view
            .filter_tags
            .as_ref()
            .expect("filtered view tags should exist")
            .normalized_text,
        "Element,Relationship"
    );
}

fn assert_dynamic_and_image_views(snapshot: &strz_analysis::DocumentSnapshot) {
    let dynamic_view = find_view(snapshot, "dynamic-view");
    assert_eq!(dynamic_view.kind, ViewKind::Dynamic);
    assert_eq!(dynamic_view.dynamic_steps.len(), 1);
    match &dynamic_view.dynamic_steps[0] {
        DynamicViewStepFact::Relationship(step) => {
            assert_eq!(step.order.as_deref(), Some("1"));
            assert_eq!(step.source.normalized_text, "system");
            assert_eq!(step.destination.normalized_text, "api");
            assert_eq!(
                step.technology
                    .as_ref()
                    .expect("dynamic step technology should exist")
                    .normalized_text,
                "HTTPS"
            );
        }
        DynamicViewStepFact::RelationshipReference(_) => {
            panic!("expected explicit dynamic relationship step");
        }
    }

    let image_view = find_view(snapshot, "image-view");
    assert_eq!(image_view.kind, ViewKind::Image);
    assert_eq!(image_view.image_sources.len(), 1);
    assert_eq!(image_view.image_sources[0].kind, ImageSourceKind::PlantUml);
    assert_eq!(image_view.image_sources[0].mode, ImageSourceMode::Default);
    assert_eq!(
        image_view.image_sources[0].value.normalized_text,
        "diagram.puml"
    );
}

#[test]
fn analysis_extracts_workspace_view_and_resource_facts() {
    let snapshot = workspace_view_and_resource_snapshot();

    assert_workspace_sections_and_scope(&snapshot);
    assert_resource_selector_and_property_facts(&snapshot);
    assert_container_and_filtered_views(&snapshot);
    assert_dynamic_and_image_views(&snapshot);
}

#[test]
fn analysis_extracts_nested_image_sources_and_dynamic_reference_steps() {
    let snapshot = analyze(indoc! {r#"
        workspace {
            model {
                user = person "User"
                system = softwareSystem "System"
                rel = user -> system "Uses"
            }

            views {
                dynamic * "dynamic-view" {
                    rel "Uses"
                    {
                        1: user -> system "Fallback" "HTTPS"
                    }
                }

                image * "image-view" {
                    light {
                        image "light.png"
                    }

                    dark {
                        kroki mermaid "dark.mmd"
                    }
                }
            }
        }
    "#});

    let dynamic_view = snapshot
        .view_facts()
        .iter()
        .find(|view| view.kind == ViewKind::Dynamic)
        .expect("dynamic view should exist");
    assert_eq!(dynamic_view.dynamic_steps.len(), 2);
    match &dynamic_view.dynamic_steps[0] {
        DynamicViewStepFact::RelationshipReference(step) => {
            assert_eq!(step.relationship.normalized_text, "rel");
            assert_eq!(step.description.normalized_text, "Uses");
        }
        DynamicViewStepFact::Relationship(_) => {
            panic!("expected relationship reference step first");
        }
    }
    match &dynamic_view.dynamic_steps[1] {
        DynamicViewStepFact::Relationship(step) => {
            assert_eq!(step.order.as_deref(), Some("1"));
            assert_eq!(
                step.technology
                    .as_ref()
                    .map(|value| value.normalized_text.as_str()),
                Some("HTTPS")
            );
        }
        DynamicViewStepFact::RelationshipReference(_) => {
            panic!("expected explicit relationship step second");
        }
    }

    let image_view = snapshot
        .view_facts()
        .iter()
        .find(|view| view.kind == ViewKind::Image)
        .expect("image view should exist");
    assert_eq!(
        image_view
            .image_sources
            .iter()
            .map(|source| (
                source.mode,
                source.kind,
                source.value.normalized_text.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (ImageSourceMode::Light, ImageSourceKind::Image, "light.png"),
            (ImageSourceMode::Dark, ImageSourceKind::Kroki, "dark.mmd"),
        ]
    );
    assert_eq!(
        image_view.image_sources[1]
            .format
            .as_ref()
            .expect("kroki format should exist")
            .normalized_text,
        "mermaid"
    );
}
