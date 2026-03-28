use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::any;
use proptest::strategy::{Just, Strategy};
use proptest::test_runner::{Config, TestCaseError};
use structurizr_analysis::{
    IncludeDiagnosticKind, WorkspaceDocumentKind, WorkspaceFacts, WorkspaceIncludeTarget,
    load_workspace,
};
use tempfile::TempDir;

#[derive(Debug, Clone, Copy)]
enum IncludeScenario {
    LocalModel,
    MissingLocal,
    Remote,
    Cycle,
}

#[derive(Debug, Clone)]
struct GeneratedWorkspaceGraph {
    scenario: IncludeScenario,
    include_unrelated_neighbor: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct WorkspaceView {
    documents: Vec<WorkspaceDocumentView>,
    includes: Vec<WorkspaceIncludeView>,
    diagnostics: Vec<WorkspaceDiagnosticView>,
}

#[derive(Debug, PartialEq, Eq)]
struct WorkspaceDocumentView {
    path: String,
    kind: WorkspaceDocumentKind,
    discovered_by_scan: bool,
    include_targets: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct WorkspaceIncludeView {
    including_document: String,
    target_text: String,
    target: WorkspaceIncludeTargetView,
    discovered_documents: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct WorkspaceDiagnosticView {
    document: String,
    kind: IncludeDiagnosticKind,
    target_text: String,
}

#[derive(Debug, PartialEq, Eq)]
enum WorkspaceIncludeTargetView {
    LocalFile { path: String },
    LocalDirectory { path: String },
    RemoteUrl { url: String },
    MissingLocalPath { path: String },
    UnsupportedLocalPath { path: String },
}

struct MaterializedWorkspace {
    _root_dir: TempDir,
    root_path: PathBuf,
    workspace_path: PathBuf,
    secondary_root: Option<PathBuf>,
}

impl WorkspaceView {
    fn from_facts(facts: &WorkspaceFacts, root: &Path) -> Self {
        let documents = facts
            .documents()
            .iter()
            .map(|document| WorkspaceDocumentView {
                path: display_path(
                    document
                        .snapshot()
                        .location()
                        .expect("generated workspace documents should have locations")
                        .path(),
                    root,
                ),
                kind: document.kind(),
                discovered_by_scan: document.discovered_by_scan(),
                include_targets: document
                    .snapshot()
                    .include_directives()
                    .iter()
                    .map(|directive| directive.raw_value.clone())
                    .collect(),
            })
            .collect();

        let includes = facts
            .includes()
            .iter()
            .map(|include| WorkspaceIncludeView {
                including_document: display_path(
                    Path::new(include.including_document().as_str()),
                    root,
                ),
                target_text: include.target_text().to_owned(),
                target: WorkspaceIncludeTargetView::from(include.target(), root),
                discovered_documents: include
                    .discovered_documents()
                    .iter()
                    .map(|document_id| display_path(Path::new(document_id.as_str()), root))
                    .collect(),
            })
            .collect();

        let diagnostics = facts
            .include_diagnostics()
            .iter()
            .map(|diagnostic| WorkspaceDiagnosticView {
                document: display_path(Path::new(diagnostic.document.as_str()), root),
                kind: diagnostic.kind,
                target_text: diagnostic.target_text.clone(),
            })
            .collect();

        Self {
            documents,
            includes,
            diagnostics,
        }
    }
}

impl WorkspaceIncludeTargetView {
    fn from(target: &WorkspaceIncludeTarget, root: &Path) -> Self {
        match target {
            WorkspaceIncludeTarget::LocalFile { path } => Self::LocalFile {
                path: display_path(path, root),
            },
            WorkspaceIncludeTarget::LocalDirectory { path } => Self::LocalDirectory {
                path: display_path(path, root),
            },
            WorkspaceIncludeTarget::RemoteUrl { url } => Self::RemoteUrl { url: url.clone() },
            WorkspaceIncludeTarget::MissingLocalPath { path } => Self::MissingLocalPath {
                path: display_path(path, root),
            },
            WorkspaceIncludeTarget::UnsupportedLocalPath { path } => Self::UnsupportedLocalPath {
                path: display_path(path, root),
            },
        }
    }
}

fn include_scenario() -> impl Strategy<Value = IncludeScenario> {
    proptest::prop_oneof![
        Just(IncludeScenario::LocalModel),
        Just(IncludeScenario::MissingLocal),
        Just(IncludeScenario::Remote),
        Just(IncludeScenario::Cycle),
    ]
}

fn local_root_order_scenario() -> impl Strategy<Value = IncludeScenario> {
    proptest::prop_oneof![
        Just(IncludeScenario::LocalModel),
        Just(IncludeScenario::Cycle),
    ]
}

fn generated_workspace_graph() -> impl Strategy<Value = GeneratedWorkspaceGraph> {
    (include_scenario(), any::<bool>()).prop_map(|(scenario, include_unrelated_neighbor)| {
        GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor,
        }
    })
}

fn materialize_workspace(model: &GeneratedWorkspaceGraph) -> MaterializedWorkspace {
    let root_dir = tempfile::tempdir().expect("tempdir should create");
    let root_path = root_dir
        .path()
        .canonicalize()
        .expect("tempdir path should canonicalize");
    let workspace_path = root_path.join("workspace.dsl");

    match model.scenario {
        IncludeScenario::LocalModel => {
            write_file(
                &workspace_path,
                "workspace {\n    !include \"model.dsl\"\n}\n",
            );
            write_file(
                &root_path.join("model.dsl"),
                "model {\n    user = person \"User\"\n    system = softwareSystem \"System\"\n    user -> system \"Uses\"\n}\n",
            );
        }
        IncludeScenario::MissingLocal => {
            write_file(
                &workspace_path,
                "workspace {\n    !include \"missing.dsl\"\n}\n",
            );
        }
        IncludeScenario::Remote => {
            write_file(
                &workspace_path,
                "workspace {\n    !include \"https://example.com/base.dsl\"\n}\n",
            );
        }
        IncludeScenario::Cycle => {
            write_file(
                &workspace_path,
                "workspace {\n    !include \"loop-a.dsl\"\n}\n",
            );
            write_file(
                &root_path.join("loop-a.dsl"),
                "model {\n    !include \"loop-b.dsl\"\n    user = person \"User\"\n}\n",
            );
            write_file(
                &root_path.join("loop-b.dsl"),
                "model {\n    !include \"loop-a.dsl\"\n    system = softwareSystem \"System\"\n}\n",
            );
        }
    }

    if model.include_unrelated_neighbor {
        write_file(
            &root_path.join("ignored.dsl"),
            "workspace {\n    model {\n        stray = person \"Stray\"\n    }\n}\n",
        );
    }

    let secondary_root = match model.scenario {
        IncludeScenario::LocalModel => Some(root_path.join("model.dsl")),
        IncludeScenario::Cycle => Some(root_path.join("loop-a.dsl")),
        IncludeScenario::MissingLocal | IncludeScenario::Remote => None,
    };

    MaterializedWorkspace {
        _root_dir: root_dir,
        root_path,
        workspace_path,
        secondary_root,
    }
}

fn write_file(path: &Path, source: &str) {
    fs::write(path, source).unwrap_or_else(|error| {
        panic!(
            "failed to write generated workspace fixture `{}`: {error}",
            path.display()
        )
    });
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn workspace_view_for(root: &Path, roots: &[&Path]) -> WorkspaceView {
    let facts = load_workspace(roots).expect("generated workspace should load");
    WorkspaceView::from_facts(&facts, root)
}

fn proptest_config() -> Config {
    let mut config = Config::default();

    if env::var_os("PROPTEST_CASES").is_none() {
        config.cases = 32;
    }

    config
}

fn maybe_capture_workspace(test_name: &str, fixture: &MaterializedWorkspace) {
    let Some(capture_dir) = env::var_os("STRUCTURIZR_PROPTEST_CAPTURE_DIR").map(PathBuf::from)
    else {
        return;
    };

    let target_dir = capture_dir.join(test_name);
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).expect("capture directory should replace");
    }

    fs::create_dir_all(&target_dir).expect("capture directory should create");
    copy_tree(&fixture.root_path, &target_dir);
}

fn copy_tree(source_root: &Path, target_root: &Path) {
    for entry in fs::read_dir(source_root).expect("workspace fixture root should read") {
        let entry = entry.expect("workspace fixture entry should read");
        let source_path = entry.path();
        let target_path = target_root.join(entry.file_name());
        let file_type = entry
            .file_type()
            .expect("workspace fixture file type should read");

        if file_type.is_dir() {
            fs::create_dir_all(&target_path).expect("capture subdirectory should create");
            copy_tree(&source_path, &target_path);
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path).unwrap_or_else(|error| {
                panic!(
                    "failed to copy generated workspace file `{}`: {error}",
                    source_path.display()
                )
            });
        }
    }
}

fn assert_expected_diagnostics(
    model: &GeneratedWorkspaceGraph,
    facts: &WorkspaceFacts,
) -> Result<(), TestCaseError> {
    let diagnostic_kinds = facts
        .include_diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.kind)
        .collect::<Vec<_>>();

    match model.scenario {
        IncludeScenario::LocalModel => {
            proptest::prop_assert!(
                diagnostic_kinds.is_empty(),
                "local include scenario should not report diagnostics: {diagnostic_kinds:?}",
            );
        }
        IncludeScenario::MissingLocal => {
            proptest::prop_assert_eq!(
                diagnostic_kinds,
                vec![IncludeDiagnosticKind::MissingLocalTarget],
            );
        }
        IncludeScenario::Remote => {
            proptest::prop_assert_eq!(
                diagnostic_kinds,
                vec![IncludeDiagnosticKind::UnsupportedRemoteTarget],
            );
        }
        IncludeScenario::Cycle => {
            proptest::prop_assert_eq!(
                diagnostic_kinds,
                vec![
                    IncludeDiagnosticKind::IncludeCycle,
                    IncludeDiagnosticKind::IncludeCycle,
                ],
            );
        }
    }

    Ok(())
}

proptest::proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn generated_workspaces_load_idempotently(model in generated_workspace_graph()) {
        let fixture = materialize_workspace(&model);
        maybe_capture_workspace("generated_workspaces_load_idempotently", &fixture);
        let first_facts = load_workspace([fixture.workspace_path.as_path()])
            .expect("generated workspace should load");
        let second_facts = load_workspace([fixture.workspace_path.as_path()])
            .expect("generated workspace should load on repeat");

        let first_view = WorkspaceView::from_facts(&first_facts, &fixture.root_path);
        let second_view = WorkspaceView::from_facts(&second_facts, &fixture.root_path);

        proptest::prop_assert_eq!(first_view, second_view);
        assert_expected_diagnostics(&model, &first_facts)?;
    }

    #[test]
    fn generated_workspace_root_order_does_not_change_results(
        scenario in local_root_order_scenario(),
        include_unrelated_neighbor in any::<bool>(),
    ) {
        let fixture = materialize_workspace(&GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor,
        });
        maybe_capture_workspace("generated_workspace_root_order_does_not_change_results", &fixture);
        let secondary_root = fixture
            .secondary_root
            .as_deref()
            .expect("local root-order scenarios should provide a secondary root");

        let left = workspace_view_for(
            &fixture.root_path,
            &[fixture.workspace_path.as_path(), secondary_root],
        );
        let right = workspace_view_for(
            &fixture.root_path,
            &[secondary_root, fixture.workspace_path.as_path()],
        );

        proptest::prop_assert_eq!(left, right);
    }

    #[test]
    fn unrelated_neighbors_do_not_change_explicit_root_loading(scenario in include_scenario()) {
        let without_unrelated = materialize_workspace(&GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor: false,
        });
        let with_unrelated = materialize_workspace(&GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor: true,
        });
        maybe_capture_workspace(
            "unrelated_neighbors_do_not_change_explicit_root_loading_without_unrelated",
            &without_unrelated,
        );
        maybe_capture_workspace(
            "unrelated_neighbors_do_not_change_explicit_root_loading_with_unrelated",
            &with_unrelated,
        );

        let baseline = workspace_view_for(
            &without_unrelated.root_path,
            &[without_unrelated.workspace_path.as_path()],
        );
        let with_neighbor = workspace_view_for(
            &with_unrelated.root_path,
            &[with_unrelated.workspace_path.as_path()],
        );

        proptest::prop_assert_eq!(baseline, with_neighbor);
    }
}
