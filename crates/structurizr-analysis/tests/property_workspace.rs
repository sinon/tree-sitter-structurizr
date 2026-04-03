use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::any;
use proptest::strategy::{Just, Strategy};
use proptest::string::string_regex;
use proptest::test_runner::{Config, TestCaseError};
use structurizr_analysis::{
    IncludeDiagnosticKind, WorkspaceDocumentKind, WorkspaceFacts, WorkspaceIncludeTarget,
    WorkspaceLoader,
};
use tempfile::TempDir;

#[derive(Debug, Clone, Copy)]
enum IncludeScenario {
    LocalModel,
    MissingLocal,
    Remote,
    Cycle,
    InheritedConstant,
    LateConstant,
}

#[derive(Debug, Clone)]
struct GeneratedWorkspaceGraph {
    scenario: IncludeScenario,
    include_unrelated_neighbor: bool,
    detail_file_name: String,
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
        Just(IncludeScenario::InheritedConstant),
        Just(IncludeScenario::LateConstant),
    ]
}

fn local_root_order_scenario() -> impl Strategy<Value = IncludeScenario> {
    proptest::prop_oneof![
        Just(IncludeScenario::LocalModel),
        Just(IncludeScenario::Cycle),
    ]
}

fn generated_workspace_graph() -> impl Strategy<Value = GeneratedWorkspaceGraph> {
    (include_scenario(), any::<bool>(), detail_file_name()).prop_map(
        |(scenario, include_unrelated_neighbor, detail_file_name)| GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor,
            detail_file_name,
        },
    )
}

fn detail_file_name() -> impl Strategy<Value = String> {
    string_regex("[a-z][a-z0-9_]{0,10}")
        .expect("detail-file regex should compile")
        .prop_map(|stem| format!("{stem}-details.dsl"))
}

fn materialize_workspace(model: &GeneratedWorkspaceGraph) -> MaterializedWorkspace {
    let root_dir = tempfile::tempdir().expect("tempdir should create");
    let root_path = root_dir
        .path()
        .canonicalize()
        .expect("tempdir path should canonicalize");
    let workspace_path = root_path.join("workspace.dsl");

    match model.scenario {
        IncludeScenario::LocalModel => materialize_local_model(&root_path, &workspace_path),
        IncludeScenario::MissingLocal => materialize_missing_local(&workspace_path),
        IncludeScenario::Remote => materialize_remote(&workspace_path),
        IncludeScenario::Cycle => materialize_cycle(&root_path, &workspace_path),
        IncludeScenario::InheritedConstant => {
            materialize_inherited_constant(model, &root_path, &workspace_path);
        }
        IncludeScenario::LateConstant => {
            materialize_late_constant(model, &root_path, &workspace_path);
        }
    }

    if model.include_unrelated_neighbor {
        write_file(
            &root_path.join("ignored.dsl"),
            "workspace {\n    model {\n        stray = person \"Stray\"\n    }\n}\n",
        );
    }

    let secondary_root = secondary_root_for(model, &root_path);

    MaterializedWorkspace {
        _root_dir: root_dir,
        root_path,
        workspace_path,
        secondary_root,
    }
}

fn materialize_local_model(root_path: &Path, workspace_path: &Path) {
    write_file(
        workspace_path,
        "workspace {\n    !include \"model.dsl\"\n}\n",
    );
    write_file(
        &root_path.join("model.dsl"),
        "model {\n    user = person \"User\"\n    system = softwareSystem \"System\"\n    user -> system \"Uses\"\n}\n",
    );
}

fn materialize_missing_local(workspace_path: &Path) {
    write_file(
        workspace_path,
        "workspace {\n    !include \"missing.dsl\"\n}\n",
    );
}

fn materialize_remote(workspace_path: &Path) {
    write_file(
        workspace_path,
        "workspace {\n    !include \"https://example.com/base.dsl\"\n}\n",
    );
}

fn materialize_cycle(root_path: &Path, workspace_path: &Path) {
    write_file(
        workspace_path,
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

fn materialize_inherited_constant(
    model: &GeneratedWorkspaceGraph,
    root_path: &Path,
    workspace_path: &Path,
) {
    let shared_dir = root_path.join("shared");
    let details_dir = shared_dir.join("details");
    fs::create_dir_all(&details_dir).expect("generated details directory should create");

    write_file(
        workspace_path,
        &format!(
            "!const DETAIL_FILE \"{}\"\n\nworkspace {{\n    model {{\n        !include shared/system.dsl\n    }}\n}}\n",
            model.detail_file_name
        ),
    );
    write_file(
        &shared_dir.join("system.dsl"),
        "system = softwareSystem \"Payments\" {\n    !include \"details/${DETAIL_FILE}\"\n}\n",
    );
    write_file(
        &details_dir.join(&model.detail_file_name),
        "api = container \"API\"\n",
    );
}

fn materialize_late_constant(
    model: &GeneratedWorkspaceGraph,
    root_path: &Path,
    workspace_path: &Path,
) {
    let shared_dir = root_path.join("shared");
    let details_dir = shared_dir.join("details");
    fs::create_dir_all(&details_dir).expect("generated details directory should create");

    write_file(
        workspace_path,
        "workspace {\n    model {\n        !include shared/system.dsl\n    }\n}\n",
    );
    write_file(
        &shared_dir.join("system.dsl"),
        &format!(
            "system = softwareSystem \"Ordered\" {{\n    !include \"details/${{DETAIL_FILE}}\"\n    !const DETAIL_FILE \"{}\"\n}}\n",
            model.detail_file_name
        ),
    );
    write_file(
        &details_dir.join(&model.detail_file_name),
        "api = container \"API\"\n",
    );
}

fn secondary_root_for(model: &GeneratedWorkspaceGraph, root_path: &Path) -> Option<PathBuf> {
    match model.scenario {
        IncludeScenario::LocalModel => Some(root_path.join("model.dsl")),
        IncludeScenario::Cycle => Some(root_path.join("loop-a.dsl")),
        IncludeScenario::InheritedConstant | IncludeScenario::LateConstant => {
            Some(root_path.join("shared/system.dsl"))
        }
        IncludeScenario::MissingLocal | IncludeScenario::Remote => None,
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
    let mut loader = WorkspaceLoader::new();
    let facts = loader
        .load_paths(roots)
        .expect("generated workspace should load");
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
        IncludeScenario::InheritedConstant => {
            proptest::prop_assert!(
                diagnostic_kinds.is_empty(),
                "inherited constants should resolve includes cleanly: {diagnostic_kinds:?}",
            );
        }
        IncludeScenario::LateConstant => {
            proptest::prop_assert_eq!(
                diagnostic_kinds,
                vec![IncludeDiagnosticKind::MissingLocalTarget],
            );
        }
    }

    Ok(())
}

fn assert_expected_constant_resolution(
    model: &GeneratedWorkspaceGraph,
    facts: &WorkspaceFacts,
    root: &Path,
) -> Result<(), TestCaseError> {
    match model.scenario {
        IncludeScenario::InheritedConstant => {
            let expected_target = format!("details/{}", model.detail_file_name);
            let include = facts
                .includes()
                .iter()
                .find(|include| include.target_text() == expected_target)
                .expect("inherited-constant scenario should resolve the nested include");
            let discovered_documents = include
                .discovered_documents()
                .iter()
                .map(|document_id| display_path(Path::new(document_id.as_str()), root))
                .collect::<Vec<_>>();

            proptest::prop_assert_eq!(
                discovered_documents,
                vec![format!("shared/{expected_target}")],
            );
        }
        IncludeScenario::LateConstant => {
            let include = facts
                .includes()
                .iter()
                .find(|include| include.raw_value() == "\"details/${DETAIL_FILE}\"")
                .expect("late-constant scenario should keep the unresolved nested include");

            proptest::prop_assert_eq!(include.target_text(), "details/${DETAIL_FILE}");
            proptest::prop_assert!(include.discovered_documents().is_empty());
        }
        IncludeScenario::LocalModel
        | IncludeScenario::MissingLocal
        | IncludeScenario::Remote
        | IncludeScenario::Cycle => {}
    }

    Ok(())
}

proptest::proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn generated_workspaces_load_idempotently(model in generated_workspace_graph()) {
        let fixture = materialize_workspace(&model);
        maybe_capture_workspace("generated_workspaces_load_idempotently", &fixture);
        let mut loader = WorkspaceLoader::new();
        let first_facts = loader
            .load_paths([fixture.workspace_path.as_path()])
            .expect("generated workspace should load");
        let second_facts = loader
            .load_paths([fixture.workspace_path.as_path()])
            .expect("generated workspace should load on repeat");

        let first_view = WorkspaceView::from_facts(&first_facts, &fixture.root_path);
        let second_view = WorkspaceView::from_facts(&second_facts, &fixture.root_path);

        proptest::prop_assert_eq!(first_view, second_view);
        assert_expected_diagnostics(&model, &first_facts)?;
        assert_expected_constant_resolution(&model, &first_facts, &fixture.root_path)?;
    }

    #[test]
    fn generated_workspace_root_order_does_not_change_results(
        scenario in local_root_order_scenario(),
        include_unrelated_neighbor in any::<bool>(),
        detail_file_name in detail_file_name(),
    ) {
        let fixture = materialize_workspace(&GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor,
            detail_file_name,
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
    fn unrelated_neighbors_do_not_change_explicit_root_loading(
        scenario in include_scenario(),
        detail_file_name in detail_file_name(),
    ) {
        let without_unrelated = materialize_workspace(&GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor: false,
            detail_file_name: detail_file_name.clone(),
        });
        let with_unrelated = materialize_workspace(&GeneratedWorkspaceGraph {
            scenario,
            include_unrelated_neighbor: true,
            detail_file_name,
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
