use std::path::{Path, PathBuf};

#[path = "../../../tests/support/repo_local_temp_workspace.rs"]
mod repo_local_temp_workspace;

use repo_local_temp_workspace::RepoLocalTempWorkspace;
use rstest::rstest;
use strz_analysis::{
    TextSpan, WorkspaceDocumentKind, WorkspaceFacts, WorkspaceIncludeTarget,
    WorkspaceLoadFailureKind, WorkspaceLoader,
};

macro_rules! set_snapshot_suffix {
    ($($expr:expr),* $(,)?) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_suffix(format!($($expr),*));
        let _guard = settings.bind_to_scope();
    };
}

#[allow(dead_code)]
#[derive(Debug)]
struct WorkspaceView {
    documents: Vec<WorkspaceDocumentView>,
    includes: Vec<WorkspaceIncludeView>,
    diagnostics: Vec<WorkspaceDiagnosticView>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct WorkspaceDocumentView {
    path: String,
    kind: WorkspaceDocumentKind,
    discovered_by_scan: bool,
    include_targets: Vec<String>,
    symbol_bindings: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct WorkspaceIncludeView {
    including_document: String,
    raw_value: String,
    target_text: String,
    target: WorkspaceIncludeTargetView,
    discovered_documents: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct WorkspaceDiagnosticView {
    document: String,
    code: String,
    message: String,
    target_text: String,
    span: TextSpan,
    value_span: Option<TextSpan>,
}

#[allow(dead_code)]
#[derive(Debug)]
enum WorkspaceIncludeTargetView {
    LocalFile { path: String },
    LocalDirectory { path: String },
    RemoteUrl { url: String },
    MissingLocalPath { path: String },
    UnsupportedLocalPath { path: String },
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
                        .expect("workspace discovery should attach a document location")
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
                symbol_bindings: document
                    .snapshot()
                    .symbols()
                    .iter()
                    .filter_map(|symbol| symbol.binding_name.clone())
                    .collect(),
            })
            .collect();

        let includes = facts
            .includes()
            .iter()
            .map(|include| WorkspaceIncludeView {
                including_document: display_document_id(
                    include.including_document().as_str(),
                    root,
                ),
                raw_value: include.raw_value().to_owned(),
                target_text: include.target_text().to_owned(),
                target: WorkspaceIncludeTargetView::from(include.target(), root),
                discovered_documents: include
                    .discovered_documents()
                    .iter()
                    .map(|document_id| display_document_id(document_id.as_str(), root))
                    .collect(),
            })
            .collect();

        let diagnostics = facts
            .include_diagnostics()
            .iter()
            .map(|diagnostic| WorkspaceDiagnosticView {
                document: display_document_id(
                    diagnostic
                        .document()
                        .expect("include diagnostics should carry documents")
                        .as_str(),
                    root,
                ),
                code: diagnostic.code().to_owned(),
                message: diagnostic.message().to_owned(),
                target_text: diagnostic
                    .target_text()
                    .expect("include diagnostics should carry target text")
                    .to_owned(),
                span: diagnostic.span(),
                value_span: diagnostic.value_span(),
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

#[rstest]
#[case("minimal-scan")]
#[case("ignored-explicit")]
#[case("directory-include")]
#[case("inherited-constants")]
#[case("remote-include")]
#[case("missing-include")]
#[case("unsupported-escape")]
#[case("cycle")]
fn workspace_fixtures_produce_stable_discovery_views(#[case] fixture_name: &str) {
    let fixture_root = workspace_fixture_root().join(fixture_name);
    let mut loader = WorkspaceLoader::new();
    let facts = loader
        .load_paths([fixture_root.as_path()])
        .unwrap_or_else(|error| {
            panic!("failed to load workspace fixture `{fixture_name}`: {error}")
        });

    set_snapshot_suffix!("{}", fixture_name.replace('-', "_"));
    insta::assert_debug_snapshot!(
        "workspace_discovery",
        WorkspaceView::from_facts(&facts, &fixture_root)
    );
}

#[test]
fn explicit_file_roots_are_loaded_even_without_dsl_extensions() {
    let explicit_file = workspace_fixture_root().join("ignored-explicit/ignored/model.inc");
    let mut loader = WorkspaceLoader::new();
    let facts = loader
        .load_paths([explicit_file.as_path()])
        .expect("explicit non-.dsl file roots should still load");

    assert_eq!(facts.documents().len(), 1);
    let document = facts
        .documents()
        .first()
        .expect("explicit file root should produce one workspace document");
    assert_eq!(document.kind(), WorkspaceDocumentKind::Fragment);
    assert_eq!(
        document
            .snapshot()
            .symbols()
            .iter()
            .filter_map(|symbol| symbol.binding_name.as_deref())
            .collect::<Vec<_>>(),
        vec!["user"]
    );
}

#[test]
fn constants_must_be_defined_before_they_can_drive_include_resolution() {
    let fixture_root = workspace_fixture_root().join("ordered-constants");
    let mut loader = WorkspaceLoader::new();
    let facts = loader
        .load_paths([fixture_root.as_path()])
        .expect("ordered-constants fixture should load successfully");

    let diagnostics = facts.include_diagnostics().iter().collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "include.missing-local-target");
    assert_eq!(
        diagnostics[0]
            .target_text()
            .expect("include diagnostics should carry target text"),
        "details/${DETAIL_FILE}"
    );
    assert_eq!(
        display_document_id(
            diagnostics[0]
                .document()
                .expect("include diagnostics should carry documents")
                .as_str(),
            &fixture_root,
        ),
        "shared/system.dsl"
    );
}

#[test]
fn workspace_base_load_failures_preserve_source_anchors() {
    let workspace = RepoLocalTempWorkspace::new("workspace-discovery", "workspace-base-missing");
    workspace.write_file(
        "workspace.dsl",
        "workspace extends \"missing-base.dsl\" {\n}\n",
    );

    let mut loader = WorkspaceLoader::new();
    let error = loader
        .load_paths_with_failures([workspace.path()])
        .expect_err("missing workspace base should abort the workspace load");

    let failures = error.failures();
    assert_eq!(failures.len(), 1);
    let failure = &failures[0];
    assert_eq!(failure.kind(), WorkspaceLoadFailureKind::WorkspaceBase);
    assert_eq!(
        failure.message(),
        "workspace base does not exist: missing-base.dsl"
    );

    let anchor = failure
        .anchor()
        .expect("workspace base failures should carry the extends value anchor");
    assert_eq!(
        display_document_id(anchor.document().as_str(), workspace.path()),
        "workspace.dsl"
    );
    assert_eq!(anchor.target_text(), Some("missing-base.dsl"));
    assert_eq!(anchor.span().start_point.row, 0);

    let diagnostic = failure
        .diagnostic()
        .expect("anchored load failures should convert to diagnostics");
    assert_eq!(diagnostic.code(), "workspace.load-failure");
    assert_eq!(
        diagnostic
            .document()
            .map(|document| display_document_id(document.as_str(), workspace.path())),
        Some("workspace.dsl".to_owned())
    );
}

#[test]
fn include_load_failures_preserve_source_anchors() {
    let workspace = RepoLocalTempWorkspace::new("workspace-discovery", "include-invalid-utf8");
    workspace.write_file("workspace.dsl", "workspace {\n  !include \"bad.inc\"\n}\n");
    workspace.write_bytes("bad.inc", &[0xff, 0xfe]);

    let mut loader = WorkspaceLoader::new();
    let error = loader
        .load_paths_with_failures([workspace.path()])
        .expect_err("invalid UTF-8 include should abort the workspace load");

    let failures = error.failures();
    assert_eq!(failures.len(), 1);
    let failure = &failures[0];
    assert_eq!(failure.kind(), WorkspaceLoadFailureKind::IncludeLoad);
    assert!(
        failure.message().contains("failed to load include bad.inc"),
        "unexpected include-load failure message: {}",
        failure.message()
    );

    let anchor = failure
        .anchor()
        .expect("include load failures should carry the include directive anchor");
    assert_eq!(
        display_document_id(anchor.document().as_str(), workspace.path()),
        "workspace.dsl"
    );
    assert_eq!(anchor.target_text(), Some("bad.inc"));
    assert_eq!(anchor.span().start_point.row, 1);
}

#[test]
fn root_load_failures_remain_unanchored_session_failures() {
    let missing_root = workspace_fixture_root().join("does-not-exist");
    let mut loader = WorkspaceLoader::new();
    let error = loader
        .load_paths_with_failures([missing_root.as_path()])
        .expect_err("missing workspace roots should abort the workspace load");

    let failures = error.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].kind(), WorkspaceLoadFailureKind::WorkspaceRoot);
    assert!(failures[0].anchor().is_none());
    assert!(
        failures[0]
            .message()
            .contains("failed to load workspace root"),
        "unexpected root-load failure message: {}",
        failures[0].message()
    );
}

fn workspace_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/lsp/workspaces")
        .canonicalize()
        .expect("workspace fixture root should exist")
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
