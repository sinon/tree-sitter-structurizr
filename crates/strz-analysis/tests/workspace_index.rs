use std::{
    fs, io,
    path::{Path, PathBuf},
};

use indoc::indoc;
use rstest::rstest;
use strz_analysis::{
    DiagnosticSeverity, ReferenceHandle, ReferenceResolutionStatus, RuledDiagnostic, SymbolHandle,
    TextSpan, WorkspaceFacts, WorkspaceLoader,
};
use tempfile::TempDir;

macro_rules! set_snapshot_suffix {
    ($($expr:expr),* $(,)?) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_suffix(format!($($expr),*));
        let _guard = settings.bind_to_scope();
    };
}

#[allow(dead_code)]
#[derive(Debug)]
struct WorkspaceIndexSetView {
    document_instances: Vec<DocumentInstanceView>,
    merged_semantic_diagnostics: Vec<DiagnosticView>,
    instances: Vec<WorkspaceIndexView>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct DocumentInstanceView {
    document: String,
    instance_ids: Vec<usize>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct WorkspaceIndexView {
    id: usize,
    root_document: String,
    documents: Vec<String>,
    unique_element_bindings: Vec<(String, String)>,
    duplicate_element_bindings: Vec<(String, Vec<String>)>,
    unique_relationship_bindings: Vec<(String, String)>,
    duplicate_relationship_bindings: Vec<(String, Vec<String>)>,
    reference_resolutions: Vec<ReferenceResolutionView>,
    semantic_diagnostics: Vec<DiagnosticView>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ReferenceResolutionView {
    reference: String,
    status: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DiagnosticView {
    document: String,
    code: String,
    message: String,
    span: TextSpan,
}

impl WorkspaceIndexSetView {
    fn from_facts(facts: &WorkspaceFacts, root: &Path) -> Self {
        let document_instances = facts
            .documents()
            .iter()
            .map(|document| DocumentInstanceView {
                document: display_document_id(document.id().as_str(), root),
                instance_ids: facts
                    .candidate_instances_for(document.id())
                    .map(|instance_id| instance_id.as_usize())
                    .collect(),
            })
            .collect();

        let merged_semantic_diagnostics = facts
            .semantic_diagnostics()
            .iter()
            .map(|diagnostic| DiagnosticView::from_diagnostic(diagnostic, root))
            .collect();

        let instances = facts
            .workspace_indexes()
            .iter()
            .map(|index| WorkspaceIndexView::from_index(facts, index, root))
            .collect();

        Self {
            document_instances,
            merged_semantic_diagnostics,
            instances,
        }
    }
}

impl WorkspaceIndexView {
    fn from_index(
        facts: &WorkspaceFacts,
        index: &strz_analysis::WorkspaceIndex,
        root: &Path,
    ) -> Self {
        let unique_element_bindings = index
            .unique_element_bindings()
            .iter()
            .map(|(key, handle)| (key.clone(), display_symbol_handle(facts, handle, root)))
            .collect();
        let duplicate_element_bindings = index
            .duplicate_element_bindings()
            .iter()
            .map(|(key, handles)| {
                (
                    key.clone(),
                    handles
                        .iter()
                        .map(|handle| display_symbol_handle(facts, handle, root))
                        .collect(),
                )
            })
            .collect();
        let unique_relationship_bindings = index
            .unique_relationship_bindings()
            .iter()
            .map(|(key, handle)| (key.clone(), display_symbol_handle(facts, handle, root)))
            .collect();
        let duplicate_relationship_bindings = index
            .duplicate_relationship_bindings()
            .iter()
            .map(|(key, handles)| {
                (
                    key.clone(),
                    handles
                        .iter()
                        .map(|handle| display_symbol_handle(facts, handle, root))
                        .collect(),
                )
            })
            .collect();

        let mut reference_resolutions = Vec::new();
        for document_id in index.documents() {
            let snapshot = facts
                .document(document_id)
                .expect("workspace index document should exist")
                .snapshot();
            for (reference_index, _) in snapshot.references().iter().enumerate() {
                let handle = ReferenceHandle::new(document_id.clone(), reference_index);
                let status = index
                    .reference_resolution(&handle)
                    .expect("workspace index should record every reference");
                reference_resolutions.push(ReferenceResolutionView {
                    reference: display_reference_handle(facts, &handle, root),
                    status: display_resolution_status(facts, status, root),
                });
            }
        }

        let semantic_diagnostics = index
            .semantic_diagnostics()
            .iter()
            .map(|diagnostic| DiagnosticView::from_diagnostic(diagnostic, root))
            .collect();

        Self {
            id: index.id().as_usize(),
            root_document: display_document_id(index.root_document().as_str(), root),
            documents: index
                .documents()
                .iter()
                .map(|document| display_document_id(document.as_str(), root))
                .collect(),
            unique_element_bindings,
            duplicate_element_bindings,
            unique_relationship_bindings,
            duplicate_relationship_bindings,
            reference_resolutions,
            semantic_diagnostics,
        }
    }
}

impl DiagnosticView {
    fn from_diagnostic(diagnostic: &RuledDiagnostic, root: &Path) -> Self {
        Self {
            document: display_document_id(
                diagnostic
                    .document()
                    .expect("semantic diagnostics should carry documents")
                    .as_str(),
                root,
            ),
            code: diagnostic.code().to_owned(),
            message: diagnostic.message().to_owned(),
            span: diagnostic.span(),
        }
    }
}

#[rstest]
#[case("cross-file-navigation")]
#[case("deployment-navigation")]
#[case("duplicate-bindings")]
#[case("hierarchical-identifiers")]
#[case("inherited-constants")]
#[case("multi-instance-open-fragment")]
fn workspace_fixtures_produce_stable_workspace_indexes(#[case] fixture_name: &str) {
    let fixture_root = workspace_fixture_root().join(fixture_name);
    let mut loader = WorkspaceLoader::new();
    let facts = loader
        .load_paths([fixture_root.as_path()])
        .unwrap_or_else(|error| {
            panic!("failed to load workspace-index fixture `{fixture_name}`: {error}")
        });

    set_snapshot_suffix!("{}", fixture_name.replace('-', "_"));
    insta::assert_debug_snapshot!(
        "workspace_index",
        WorkspaceIndexSetView::from_facts(&facts, &fixture_root)
    );
}

#[test]
fn repeated_model_sections_report_related_context() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        !include "model-a.dsl"
                        !include "model-b.dsl"
                    }
                "#},
            ),
            (
                "model-a.dsl",
                indoc! {r#"
                    model {
                        user = person "User"
                    }
                "#},
            ),
            (
                "model-b.dsl",
                indoc! {r#"
                    model {
                        admin = person "Admin"
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.repeated-workspace-section");
    assert_eq!(diagnostics.len(), 1);

    let diagnostic = diagnostics[0];
    assert_eq!(
        display_document_id(
            diagnostic
                .document()
                .expect("workspace-section diagnostics should carry a document")
                .as_str(),
            workspace.root(),
        ),
        "model-b.dsl"
    );
    assert_eq!(
        diagnostic.message(),
        "multiple model sections are not permitted in a DSL definition"
    );
    assert_eq!(diagnostic.annotations().len(), 1);
    assert_eq!(
        diagnostic.annotations()[0]
            .document
            .as_ref()
            .map(|document| display_document_id(document.as_str(), workspace.root())),
        Some("model-a.dsl".to_owned())
    );
    assert_eq!(
        diagnostic.annotations()[0].message.as_deref(),
        Some("first model section here")
    );
}

#[test]
fn duplicate_bindings_report_conflicting_declarations_as_annotations() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        !include "alpha.dsl"
                        !include "beta.dsl"
                    }
                "#},
            ),
            (
                "alpha.dsl",
                indoc! {r#"
                    model {
                        api = softwareSystem "Alpha API"
                    }
                "#},
            ),
            (
                "beta.dsl",
                indoc! {r#"
                    model {
                        api = softwareSystem "Beta API"
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.duplicate-binding");
    assert_eq!(diagnostics.len(), 2);

    let alpha_diagnostic = diagnostics
        .iter()
        .copied()
        .find(|diagnostic| {
            display_document_id(
                diagnostic
                    .document()
                    .expect("duplicate binding diagnostics should carry documents")
                    .as_str(),
                workspace.root(),
            ) == "alpha.dsl"
        })
        .expect("alpha duplicate binding diagnostic should exist");
    assert_eq!(alpha_diagnostic.message(), "duplicate element binding: api");
    assert_eq!(alpha_diagnostic.annotations().len(), 1);
    assert_eq!(
        alpha_diagnostic.annotations()[0]
            .document
            .as_ref()
            .map(|document| display_document_id(document.as_str(), workspace.root())),
        Some("beta.dsl".to_owned())
    );
    assert_eq!(
        alpha_diagnostic.annotations()[0].message.as_deref(),
        Some("other element binding for api is declared here")
    );
}

#[test]
fn reference_diagnostics_explain_unresolved_and_ambiguous_resolution_states() {
    let (_workspace, unresolved_facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        user -> api "Calls"
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );
    let unresolved = diagnostics_of_code(&unresolved_facts, "semantic.unresolved-reference");
    assert_eq!(unresolved.len(), 1);
    assert_eq!(
        unresolved[0].message(),
        "unresolved element reference: api (no matching binding found)"
    );

    let (_workspace, ambiguous_facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        !include "alpha.dsl"
                        !include "beta.dsl"

                        model {
                            user = person "User"
                            user -> api "Calls"
                        }
                    }
                "#},
            ),
            (
                "alpha.dsl",
                indoc! {r#"
                    model {
                        api = softwareSystem "Alpha API"
                    }
                "#},
            ),
            (
                "beta.dsl",
                indoc! {r#"
                    model {
                        api = softwareSystem "Beta API"
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );
    let ambiguous = diagnostics_of_code(&ambiguous_facts, "semantic.ambiguous-reference");
    assert_eq!(ambiguous.len(), 1);
    assert_eq!(
        ambiguous[0].message(),
        "ambiguous element reference: api (multiple bindings match)"
    );
}

#[test]
fn multi_context_reference_disagreements_surface_as_warnings() {
    let fixture_root = workspace_fixture_root().join("multi-instance-open-fragment");
    let mut loader = WorkspaceLoader::new();
    let facts = loader
        .load_paths([fixture_root.as_path()])
        .expect("multi-instance fixture should load");

    let diagnostics = diagnostics_of_code(&facts, "semantic.multi-context-disagreement");
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() == DiagnosticSeverity::Warning)
    );
    let model_diagnostic = diagnostics
        .iter()
        .copied()
        .find(|diagnostic| {
            diagnostic.message()
                == "workspace contexts report different details for: multiple model sections are not permitted in a DSL definition (reported in all 2 contexts)"
        })
        .expect("repeated model section disagreement warning should exist");
    assert_eq!(
        display_document_id(
            model_diagnostic
                .document()
                .expect("multi-context diagnostics should carry documents")
                .as_str(),
            &fixture_root,
        ),
        "shared/model.dsl"
    );
    assert_eq!(model_diagnostic.annotations().len(), 2);

    let view_diagnostic = diagnostics
        .iter()
        .copied()
        .find(|diagnostic| {
            diagnostic.message()
                == "some workspace contexts report: unresolved element or relationship reference: api (no matching binding found) (reported in 1 of 2 contexts)"
        })
        .expect("context-specific unresolved reference warning should exist");
    assert_eq!(
        display_document_id(
            view_diagnostic
                .document()
                .expect("multi-context diagnostics should carry documents")
                .as_str(),
            &fixture_root,
        ),
        "shared/view.dsl"
    );
    assert_eq!(
        view_diagnostic.message(),
        "some workspace contexts report: unresolved element or relationship reference: api (no matching binding found) (reported in 1 of 2 contexts)"
    );
}

#[test]
fn workspace_scope_mismatch_reports_offending_owner() {
    let (workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "System" {
                            app = container "App"
                        }
                    }

                    configuration {
                        scope landscape
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.workspace-scope-mismatch");
    assert_eq!(diagnostics.len(), 1);

    let diagnostic = diagnostics[0];
    assert_eq!(
        display_document_id(
            diagnostic
                .document()
                .expect("workspace-scope diagnostics should carry a document")
                .as_str(),
            workspace.root(),
        ),
        "workspace.dsl"
    );
    assert_eq!(
        diagnostic.message(),
        "workspace is landscape scoped, but the software system named System has containers"
    );
    assert_eq!(diagnostic.annotations().len(), 1);
    assert!(diagnostic.annotations()[0].document.is_none());
    assert_eq!(
        diagnostic.annotations()[0].message.as_deref(),
        Some("software system named System has containers")
    );
}

#[test]
fn root_workspace_scope_wins_over_included_fragment_scope() {
    let (_workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        !include "scope.dsl"

                        model {
                            system = softwareSystem "System" {
                                app = container "App"
                            }
                        }

                        configuration {
                            scope softwareSystem
                        }
                    }
                "#},
            ),
            (
                "scope.dsl",
                indoc! {r"
                    configuration {
                        scope landscape
                    }
                "},
            ),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.workspace-scope-mismatch");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn workspace_scope_mismatch_reports_each_violating_owner_once() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        payments = softwareSystem "Payments" {
                            api = container "API"
                        }

                        accounts = softwareSystem "Accounts" {
                            ui = container "UI"
                        }
                    }

                    configuration {
                        scope landscape
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.workspace-scope-mismatch");
    assert_eq!(diagnostics.len(), 2);
    let mut messages = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message())
        .collect::<Vec<_>>();
    messages.sort_unstable();
    assert_eq!(
        messages,
        vec![
            "workspace is landscape scoped, but the software system named Accounts has containers",
            "workspace is landscape scoped, but the software system named Payments has containers",
        ]
    );
}

#[test]
fn extended_workspaces_inherit_base_bindings_without_foundation_diagnostics() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "base.dsl",
                indoc! {r#"
                    workspace {
                        !identifiers hierarchical
                        !include "deployment.dsl"
                    }
                "#},
            ),
            (
                "deployment.dsl",
                indoc! {r#"
                    model {
                        live = deploymentEnvironment "Live" {
                            aws = deploymentNode "AWS" {
                                region = deploymentNode "Region" {
                                    route53 = infrastructureNode "Route 53"
                                }
                            }
                        }
                    }
                "#},
            ),
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace extends "base.dsl" {
                        model {
                            !element live.aws.region {
                                extra = infrastructureNode "Extra" {
                                    -> route53
                                }
                            }
                        }
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );

    let diagnostics = facts
        .semantic_diagnostics()
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.code(),
                "semantic.repeated-workspace-section"
                    | "semantic.unresolved-element-selector"
                    | "semantic.unresolved-reference"
                    | "semantic.ambiguous-reference"
            )
        })
        .collect::<Vec<_>>();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let root_document = workspace.root().join("workspace.dsl");
    let index = facts
        .workspace_indexes()
        .iter()
        .find(|index| index.root_document().as_str() == root_document.to_string_lossy())
        .expect("explicit workspace root should produce one workspace index");
    assert!(index.unique_deployment_bindings().contains_key("live"));
    assert!(
        index
            .unique_deployment_bindings()
            .contains_key("live.aws.region.route53")
    );
}

#[test]
fn deployment_parent_child_relationships_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        live = deploymentEnvironment "Live" {
                            primary = deploymentNode "Primary" {
                                gateway = infrastructureNode "Gateway"
                                apiInstance = containerInstance api
                            }

                            primary -> gateway "Hosts traffic"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.deployment-parent-child-relationship");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        "Relationships cannot be added between parents and children"
    );
    assert_eq!(diagnostics[0].annotations().len(), 2);
    assert_eq!(
        diagnostics[0].annotations()[0].message.as_deref(),
        Some("ancestor deployment element Primary is declared here")
    );
    assert_eq!(
        diagnostics[0].annotations()[1].message.as_deref(),
        Some("descendant deployment element Gateway is declared here")
    );
}

#[test]
fn deployment_grandparent_relationships_also_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        live = deploymentEnvironment "Live" {
                            primary = deploymentNode "Primary" {
                                zone = deploymentNode "Zone" {
                                    gateway = infrastructureNode "Gateway"
                                }
                            }

                            primary -> gateway "Routes traffic"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.deployment-parent-child-relationship");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].annotations()[0].message.as_deref(),
        Some("ancestor deployment element Primary is declared here")
    );
    assert_eq!(
        diagnostics[0].annotations()[1].message.as_deref(),
        Some("descendant deployment element Gateway is declared here")
    );
}

#[test]
fn deployment_node_to_instance_relationships_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        live = deploymentEnvironment "Live" {
                            primary = deploymentNode "Primary" {
                                apiInstance = containerInstance api
                                systemInstance = softwareSystemInstance system
                            }

                            primary -> apiInstance "Hosts API"
                            primary -> systemInstance "Hosts system"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.deployment-parent-child-relationship");
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.message() == "Relationships cannot be added between parents and children"
            && diagnostic.annotations().len() == 2
            && diagnostic.annotations()[0].message.as_deref()
                == Some("ancestor deployment element Primary is declared here")
    }));
}

#[test]
fn deployment_sibling_relationships_remain_valid() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        live = deploymentEnvironment "Live" {
                            primary = deploymentNode "Primary" {
                                gateway = infrastructureNode "Gateway"
                                apiInstance = containerInstance api {
                                    this -> gateway "Checks health"
                                    gateway -> this "Routes traffic"
                                    -> this "Implicit return path"
                                }
                                softwareSystemInstance system
                            }
                            secondary = deploymentNode "Secondary" {
                                secondaryApiInstance = containerInstance api
                            }

                            primary -> secondary "Replicates traffic"
                            gateway -> secondaryApiInstance "Routes traffic"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.deployment-parent-child-relationship");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn missing_documentation_paths_surface_semantic_diagnostics() {
    let (workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    !docs "docs"
                    !adrs "adrs"
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.invalid-documentation-path");
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>(),
        vec![
            format!(
                "Documentation path {} does not exist",
                workspace.root().join("docs").display()
            ),
            format!(
                "Documentation path {} does not exist",
                workspace.root().join("adrs").display()
            ),
        ]
    );
}

#[test]
fn adrs_paths_must_resolve_to_directories_while_docs_can_use_files() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        !docs "docs/readme.md"
                        !adrs "adrs/0001.md"
                    }
                "#},
            ),
            ("docs/readme.md", "# Readme\n"),
            ("adrs/0001.md", "# Decision\n"),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.invalid-documentation-path");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        format!(
            "Documentation path {} is not a directory",
            workspace.root().join("adrs/0001.md").display()
        )
    );
}

#[test]
fn image_source_paths_surface_file_and_directory_diagnostics() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        views {
                            properties {
                                "plantuml.url" "https://plantuml.example.com"
                            }

                            image * "image-view" {
                                plantuml "diagram.puml"
                                image "assets"
                                plantuml """
                                    @startuml
                                    Alice -> Bob
                                    @enduml
                                """
                                image "https://example.com/logo.png"
                            }
                        }
                    }
                "#},
            ),
            ("assets/.keep", ""),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.invalid-image-source");
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message())
            .collect::<Vec<_>>(),
        vec![
            format!(
                "The file at {} does not exist",
                workspace.root().join("diagram.puml").display()
            ),
            format!(
                "{} is not a file",
                workspace.root().join("assets").display()
            ),
        ]
    );
}

#[test]
fn diagram_image_sources_preserve_upstream_directory_message() {
    let (_workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        views {
                            properties {
                                "plantuml.url" "https://plantuml.example.com"
                            }

                            image * "image-view" {
                                plantuml "assets"
                            }
                        }
                    }
                "#},
            ),
            ("assets/.keep", ""),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.invalid-image-source");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message(), "Is a directory");
}

#[test]
fn image_renderer_properties_can_be_view_local_or_viewset_level() {
    let (_workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        views {
                            properties {
                                "mermaid.url" "https://mermaid.example.com"
                            }

                            image * "image-view" {
                                properties {
                                    "plantuml.url" "https://plantuml.example.com"
                                }
                                plantuml "diagram.puml"
                                mermaid "diagram.mmd"
                                kroki plantuml "diagram.kroki"
                            }
                        }
                    }
                "#},
            ),
            ("diagram.puml", "@startuml\n@enduml\n"),
            ("diagram.mmd", "graph TD\n"),
            ("diagram.kroki", "graph TD\n"),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.missing-image-renderer-property");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        "Please define a view/viewset property named kroki.url to specify your Kroki server"
    );
    let invalid_image_sources = diagnostics_of_code(&facts, "semantic.invalid-image-source");
    assert!(
        invalid_image_sources.is_empty(),
        "{invalid_image_sources:#?}"
    );
}

#[test]
fn unresolved_element_selector_targets_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    !identifiers hierarchical

                    model {
                        system = softwareSystem "System"

                        !element system.api {
                            properties {
                                "team" "Core"
                            }
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.unresolved-element-selector");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        "unresolved !element selector target: system.api"
    );
}

#[test]
fn filtered_views_with_autolayout_bases_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "System"
                    }

                    views {
                        systemLandscape landscape {
                            include system
                            autoLayout
                        }

                        filtered landscape include "Element" filtered-landscape
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.filtered-view-autolayout-mismatch");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        "The view \"landscape\" has automatic layout enabled - this is not supported for filtered views"
    );
    assert_eq!(diagnostics[0].annotations().len(), 1);
    assert_eq!(
        diagnostics[0].annotations()[0].message.as_deref(),
        Some("base view enables automatic layout here")
    );
}

#[test]
fn filtered_view_annotations_point_to_the_base_view_document() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "base.dsl",
                indoc! {r#"
                    workspace {
                        model {
                            system = softwareSystem "System"
                        }

                        views {
                            systemLandscape landscape {
                                include system
                                autoLayout
                            }
                        }
                    }
                "#},
            ),
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace extends "base.dsl" {
                        views {
                            filtered landscape include "Element" filtered-landscape
                        }
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.filtered-view-autolayout-mismatch");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].annotations()[0]
            .document
            .as_ref()
            .map(|document| display_document_id(document.as_str(), workspace.root())),
        Some("base.dsl".to_owned())
    );
}

#[test]
fn dynamic_view_relationship_technology_mismatches_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            app = container "App"
                        }

                        user -> app "Uses" "HTTP"
                    }

                    views {
                        dynamic system "dynamic-view" {
                            1: user -> app "Requests data" "HTTPS"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-relationship-mismatch");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        "A relationship between User and App with technology HTTPS does not exist in model."
    );
    assert_eq!(diagnostics[0].annotations().len(), 1);
    assert_eq!(
        diagnostics[0].annotations()[0].message.as_deref(),
        Some("declared relationship here uses technology HTTP")
    );
}

#[test]
fn dynamic_view_relationship_annotations_point_to_the_declared_relationship_document() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        !include "model.dsl"
                        !include "dynamic.dsl"
                    }
                "#},
            ),
            (
                "model.dsl",
                indoc! {r#"
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            app = container "App"
                        }

                        user -> app "Uses" "HTTP"
                    }
                "#},
            ),
            (
                "dynamic.dsl",
                indoc! {r#"
                    views {
                        dynamic system "dynamic-view" {
                            1: user -> app "Requests data" "HTTPS"
                        }
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-relationship-mismatch");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].annotations()[0]
            .document
            .as_ref()
            .map(|document| display_document_id(document.as_str(), workspace.root())),
        Some("model.dsl".to_owned())
    );
}

#[test]
fn selector_scoped_this_and_omitted_source_relationships_match_dynamic_steps() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "System" {
                            api = container "API"
                            worker = container "Worker"

                            !element api {
                                worker -> this "Targets selector"
                                -> worker "Selector source"
                            }
                        }
                    }

                    views {
                        dynamic system "dynamic-view" {
                            1: worker -> api "Targets selector"
                            2: api -> worker "Selector source"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-relationship-mismatch");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn invalid_this_references_surface_unresolved_reference_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        this -> user "Uses"
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.unresolved-reference");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].span().start_point.row, 3);
}

#[test]
fn explicit_request_then_response_stays_valid() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        user -> api "Uses"
                    }

                    views {
                        dynamic system "dynamic-view" {
                            user -> api "Uses"
                            api -> user "Responds"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-relationship-mismatch");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn relationship_reference_then_response_stays_valid() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        rel = user -> api "Uses"
                    }

                    views {
                        dynamic system "dynamic-view" {
                            rel "Uses"
                            api -> user "Responds"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-relationship-mismatch");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn response_first_still_rejects_without_a_prior_request() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        user -> api "Uses"
                    }

                    views {
                        dynamic system "dynamic-view" {
                            api -> user "Responds"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-relationship-mismatch");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        "A relationship between API and User does not exist in model."
    );
}

#[test]
fn dynamic_view_scope_steps_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        user -> api "Uses"
                        api -> user "Responds"
                    }

                    views {
                        dynamic api "dynamic-view" {
                            user -> api "Uses"
                            api -> user "Responds"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-scope-redundancy");
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.message() == "API is already the scope of this view and cannot be added to it"
            && diagnostic.annotations().len() == 1
            && diagnostic.annotations()[0].message.as_deref() == Some("view scope is declared here")
    }));

    let mismatch_diagnostics =
        diagnostics_of_code(&facts, "semantic.dynamic-view-relationship-mismatch");
    assert!(mismatch_diagnostics.is_empty(), "{mismatch_diagnostics:#?}");
}

#[test]
fn dynamic_view_scope_relationship_references_surface_semantic_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        rel = user -> api "Uses"
                    }

                    views {
                        dynamic api "dynamic-view" {
                            rel "Uses"
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-scope-redundancy");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message(),
        "API is already the scope of this view and cannot be added to it"
    );
    assert_eq!(diagnostics[0].annotations().len(), 2);
    assert_eq!(
        diagnostics[0].annotations()[0].message.as_deref(),
        Some("view scope is declared here")
    );
    assert_eq!(
        diagnostics[0].annotations()[1].message.as_deref(),
        Some("referenced relationship here already includes API")
    );
}

#[test]
fn dynamic_view_scope_reference_annotations_point_to_the_relationship_document() {
    let (workspace, facts) = load_temp_workspace(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace {
                        !include "model.dsl"
                        !include "dynamic.dsl"
                    }
                "#},
            ),
            (
                "model.dsl",
                indoc! {r#"
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            api = container "API"
                        }

                        rel = user -> api "Uses"
                    }
                "#},
            ),
            (
                "dynamic.dsl",
                indoc! {r#"
                    views {
                        dynamic api "dynamic-view" {
                            rel "Uses"
                        }
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.dynamic-view-scope-redundancy");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].annotations()[1]
            .document
            .as_ref()
            .map(|document| display_document_id(document.as_str(), workspace.root())),
        Some("model.dsl".to_owned())
    );
}

#[test]
fn invalid_container_view_elements_surface_include_and_animation_diagnostics() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "System" {
                            api = container "API" {
                                worker = component "Worker"
                            }
                        }
                    }

                    views {
                        container system "container-view" {
                            include api worker
                            animation {
                                api worker
                            }
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.invalid-view-element");
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.message() == "The element \"worker\" can not be added to this type of view"
    }));
}

#[test]
fn system_context_scope_elements_remain_valid_for_current_upstream_parity() {
    let (_workspace, facts) = load_temp_workspace(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "System" {
                            api = container "API"
                        }
                    }

                    views {
                        systemContext system "system-context" {
                            include user system
                            animation {
                                user system
                            }
                        }
                    }
                }
            "#},
        )],
        "workspace.dsl",
    );

    let diagnostics = diagnostics_of_code(&facts, "semantic.invalid-view-element");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn workspace_extends_rejects_escape_paths() {
    let (workspace, error) = load_temp_workspace_error(
        &[(
            "workspace.dsl",
            indoc! {r#"
                workspace extends "../outside/base.dsl" {
                }
            "#},
        )],
        "workspace.dsl",
    );

    assert!(
        error
            .to_string()
            .contains("workspace base path escapes the allowed subtree"),
        "unexpected error for {}: {error}",
        workspace.root().display()
    );
}

#[test]
fn workspace_extends_cycles_surface_explicit_errors() {
    let (_workspace, error) = load_temp_workspace_error(
        &[
            (
                "workspace.dsl",
                indoc! {r#"
                    workspace extends "base.dsl" {
                    }
                "#},
            ),
            (
                "base.dsl",
                indoc! {r#"
                    workspace extends "workspace.dsl" {
                    }
                "#},
            ),
        ],
        "workspace.dsl",
    );

    assert!(
        error
            .to_string()
            .contains("workspace extends cycle detected while following"),
        "unexpected error: {error}"
    );
}

fn workspace_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/lsp/workspaces")
        .canonicalize()
        .expect("workspace fixture root should exist")
}

struct TempWorkspace {
    _root_dir: TempDir,
    root: PathBuf,
}

impl TempWorkspace {
    fn root(&self) -> &Path {
        &self.root
    }
}

fn load_temp_workspace(files: &[(&str, &str)], root_file: &str) -> (TempWorkspace, WorkspaceFacts) {
    let workspace = write_temp_workspace(files);
    let root_file = workspace.root.join(root_file);
    let mut loader = WorkspaceLoader::new();
    let facts = loader
        .load_paths([root_file.as_path()])
        .expect("temp workspace should load successfully");

    (workspace, facts)
}

fn load_temp_workspace_error(
    files: &[(&str, &str)],
    root_file: &str,
) -> (TempWorkspace, io::Error) {
    let workspace = write_temp_workspace(files);
    let root_file = workspace.root.join(root_file);
    let mut loader = WorkspaceLoader::new();
    let error = loader
        .load_paths([root_file.as_path()])
        .expect_err("temp workspace should fail to load");

    (workspace, error)
}

fn write_temp_workspace(files: &[(&str, &str)]) -> TempWorkspace {
    let root_dir = tempfile::tempdir().expect("tempdir should create");
    let root = root_dir
        .path()
        .canonicalize()
        .expect("tempdir path should canonicalize");

    for (relative_path, source) in files {
        let path = root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should create");
        }
        fs::write(path, source).expect("workspace fixture file should write");
    }

    TempWorkspace {
        _root_dir: root_dir,
        root,
    }
}

fn diagnostics_of_code<'a>(facts: &'a WorkspaceFacts, code: &str) -> Vec<&'a RuledDiagnostic> {
    facts
        .semantic_diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code() == code)
        .collect()
}

fn display_symbol_handle(facts: &WorkspaceFacts, handle: &SymbolHandle, root: &Path) -> String {
    let snapshot = facts
        .document(handle.document())
        .expect("symbol-handle document should exist")
        .snapshot();
    let symbol = snapshot
        .symbols()
        .get(handle.symbol_id().0)
        .expect("symbol-handle symbol should exist");
    let label = symbol
        .binding_name
        .as_deref()
        .unwrap_or(&symbol.display_name);

    format!(
        "{}::{label}",
        display_document_id(handle.document().as_str(), root)
    )
}

fn display_reference_handle(
    facts: &WorkspaceFacts,
    handle: &ReferenceHandle,
    root: &Path,
) -> String {
    let snapshot = facts
        .document(handle.document())
        .expect("reference-handle document should exist")
        .snapshot();
    let reference = snapshot
        .references()
        .get(handle.reference_index())
        .expect("reference-handle reference should exist");

    format!(
        "{}::{}@{}:{}",
        display_document_id(handle.document().as_str(), root),
        reference.raw_text,
        reference.span.start_point.row,
        reference.span.start_point.column
    )
}

fn display_resolution_status(
    facts: &WorkspaceFacts,
    status: &ReferenceResolutionStatus,
    root: &Path,
) -> String {
    match status {
        ReferenceResolutionStatus::Resolved(handle) => {
            format!("resolved {}", display_symbol_handle(facts, handle, root))
        }
        ReferenceResolutionStatus::UnresolvedNoMatch => "unresolved".to_owned(),
        ReferenceResolutionStatus::AmbiguousDuplicateBinding => "ambiguous-duplicate".to_owned(),
        ReferenceResolutionStatus::AmbiguousElementVsRelationship => {
            "ambiguous-element-vs-relationship".to_owned()
        }
        ReferenceResolutionStatus::DeferredByScopePolicy => "deferred".to_owned(),
    }
}

fn display_document_id(document_id: &str, root: &Path) -> String {
    display_path(Path::new(document_id), root)
}

fn display_path(path: &Path, root: &Path) -> String {
    let mut candidate_root = Some(root);
    let mut parent_prefix_count = 0usize;

    while let Some(candidate) = candidate_root {
        if let Ok(relative) = path.strip_prefix(candidate) {
            return format!(
                "{}{}",
                "../".repeat(parent_prefix_count),
                relative.display()
            );
        }

        candidate_root = candidate.parent();
        parent_prefix_count += 1;
    }

    path.display().to_string()
}
