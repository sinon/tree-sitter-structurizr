use std::path::{Path, PathBuf};

use rstest::rstest;
use structurizr_analysis::{
    ReferenceHandle, ReferenceResolutionStatus, SemanticDiagnostic, SymbolHandle, TextSpan,
    WorkspaceFacts, WorkspaceLoader,
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
    kind: structurizr_analysis::SemanticDiagnosticKind,
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
        index: &structurizr_analysis::WorkspaceIndex,
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
    fn from_diagnostic(diagnostic: &SemanticDiagnostic, root: &Path) -> Self {
        Self {
            document: display_document_id(diagnostic.document.as_str(), root),
            kind: diagnostic.kind,
            message: diagnostic.message.clone(),
            span: diagnostic.span,
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

fn workspace_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/lsp/workspaces")
        .canonicalize()
        .expect("workspace fixture root should exist")
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
