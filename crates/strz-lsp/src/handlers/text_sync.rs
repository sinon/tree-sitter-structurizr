//! Full-document sync handlers that keep the latest snapshot in server state.

use std::{collections::BTreeSet, path::PathBuf, time::Instant};

use strz_analysis::{DocumentAnalyzer, DocumentId};
use tower_lsp_server::ls_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, MessageType,
};
use tracing::{debug, info, warn};

use strz_analysis::{WorkspaceFacts, WorkspaceLoadFailure, WorkspaceLoader};

use crate::{documents::DocumentState, handlers::diagnostics, server::Backend};

/// Handles `textDocument/didOpen` by analyzing and publishing the initial snapshot.
pub async fn did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
    info!(
        uri = params.text_document.uri.as_str(),
        version = params.text_document.version,
        text_bytes = params.text_document.text.len(),
        "handling didOpen"
    );
    let document = DocumentState::new(
        params.text_document.uri,
        params.text_document.version,
        params.text_document.text,
    );

    publish_latest_snapshot(backend, document).await;
}

/// Handles `textDocument/didChange` using full-document synchronization.
pub async fn did_change(backend: &Backend, params: DidChangeTextDocumentParams) {
    let Some(updated_text) = params
        .content_changes
        .into_iter()
        .last()
        .map(|change| change.text)
    else {
        debug!(
            uri = params.text_document.uri.as_str(),
            "ignoring didChange without a full-text payload"
        );
        return;
    };
    info!(
        uri = params.text_document.uri.as_str(),
        version = params.text_document.version,
        text_bytes = updated_text.len(),
        "handling didChange"
    );

    let updated_document = {
        let mut state = backend.state().write().await;
        if let Some(document) = state.documents_mut().get_mut(&params.text_document.uri) {
            document.replace_text(params.text_document.version, updated_text);
            document.clone()
        } else {
            let document = DocumentState::new(
                params.text_document.uri.clone(),
                params.text_document.version,
                updated_text,
            );
            state.documents_mut().open(document.clone());
            document
        }
    };

    publish_latest_snapshot(backend, updated_document).await;
}

/// Handles `textDocument/didClose` by clearing cached state and diagnostics.
pub async fn did_close(backend: &Backend, params: DidCloseTextDocumentParams) {
    info!(uri = params.text_document.uri.as_str(), "handling didClose");
    let closed_document = {
        let mut state = backend.state().write().await;
        let closed_document = state.documents().get(&params.text_document.uri).cloned();
        state.documents_mut().close(&params.text_document.uri);
        state.remove_snapshot(&params.text_document.uri);
        debug!(
            open_document_count = state.documents().len(),
            "removed closed document from server state"
        );
        closed_document
    };

    let workspace_load =
        recompute_workspace_facts_after_close(backend, closed_document.as_ref()).await;

    let messages = {
        let mut state = backend.state().write().await;
        state.set_workspace_facts(workspace_load.facts);
        state.set_workspace_load_failures(workspace_load.failures)
    };
    publish_workspace_load_messages(backend, messages);

    backend
        .client()
        .publish_diagnostics(params.text_document.uri, Vec::new(), None)
        .await;
    info!(
        diagnostic_count = 0,
        clear_on_close = true,
        "published clear-diagnostics notification for closed document"
    );

    publish_open_document_diagnostics(backend).await;
}

async fn publish_latest_snapshot(backend: &Backend, document: DocumentState) {
    let uri = document.uri().clone();
    let workspace_load = recompute_workspace_facts(backend, Some(&document)).await;
    // When workspace recomputation already analyzed this file-backed document
    // through `WorkspaceLoader`, reuse that snapshot instead of immediately
    // parsing and extracting the same document a second time.
    let snapshot = snapshot_from_workspace_facts(&document, workspace_load.facts.as_ref())
        .unwrap_or_else(|| {
            let mut analyzer = DocumentAnalyzer::new();
            analyzer.analyze(document.to_input())
        });
    debug!(
        uri = uri.as_str(),
        syntax_diagnostic_count = snapshot.syntax_diagnostics().len(),
        symbol_count = snapshot.symbols().len(),
        reference_count = snapshot.references().len(),
        "analyzed latest document snapshot"
    );

    let messages = {
        let mut state = backend.state().write().await;
        state.documents_mut().open(document);
        state.set_snapshot(uri.clone(), snapshot);
        state.set_workspace_facts(workspace_load.facts);
        let messages = state.set_workspace_load_failures(workspace_load.failures);
        debug!(
            uri = uri.as_str(),
            open_document_count = state.documents().len(),
            "stored latest snapshot and workspace facts"
        );
        messages
    };

    publish_workspace_load_messages(backend, messages);
    publish_open_document_diagnostics(backend).await;
}

async fn publish_open_document_diagnostics(backend: &Backend) {
    let publish_jobs = {
        let state = backend.state().read().await;
        let workspace_facts = state.workspace_facts();
        let workspace_load_failures = state.workspace_load_failures();

        state
            .documents()
            .iter()
            .filter_map(|document| {
                let snapshot = state.snapshot(document.uri())?;

                Some((
                    document.uri().clone(),
                    document.version(),
                    diagnostics::document_diagnostics(
                        document,
                        snapshot,
                        workspace_facts,
                        workspace_load_failures,
                    ),
                ))
            })
            .collect::<Vec<_>>()
    };

    for (uri, version, publishable_diagnostics) in publish_jobs {
        let diagnostic_count = publishable_diagnostics.len();
        info!(
            uri = uri.as_str(),
            version,
            diagnostic_count,
            clear_on_close = false,
            "publishing diagnostics"
        );
        backend
            .client()
            .publish_diagnostics(uri, publishable_diagnostics, Some(version))
            .await;
    }
}

fn publish_workspace_load_messages(backend: &Backend, messages: Vec<String>) {
    if messages.is_empty() {
        return;
    }

    let client = backend.client().clone();
    tokio::spawn(async move {
        for message in messages {
            client
                .show_message(MessageType::ERROR, message.clone())
                .await;
            client.log_message(MessageType::ERROR, message).await;
        }
    });
}

#[derive(Debug, Default)]
pub(super) struct WorkspaceLoadState {
    pub(super) facts: Option<WorkspaceFacts>,
    pub(super) failures: Vec<WorkspaceLoadFailure>,
}

pub(super) async fn recompute_workspace_facts(
    backend: &Backend,
    current_document: Option<&DocumentState>,
) -> WorkspaceLoadState {
    recompute_workspace_facts_for_event(backend, current_document, current_document).await
}

async fn recompute_workspace_facts_after_close(
    backend: &Backend,
    closed_document: Option<&DocumentState>,
) -> WorkspaceLoadState {
    recompute_workspace_facts_for_event(backend, None, closed_document).await
}

async fn recompute_workspace_facts_for_event(
    backend: &Backend,
    current_document: Option<&DocumentState>,
    event_document: Option<&DocumentState>,
) -> WorkspaceLoadState {
    let (reload_plan, open_documents) = {
        let state = backend.state().read().await;
        let workspace_roots = state
            .workspace_roots()
            .iter()
            .filter_map(canonical_file_path_from_uri)
            .collect::<Vec<_>>();
        let open_documents = state.documents().iter().cloned().collect::<Vec<_>>();

        (
            WorkspaceReloadPlan::new(
                &workspace_roots,
                &open_documents,
                state.workspace_facts(),
                state.workspace_load_failures(),
                event_document,
            ),
            open_documents,
        )
    };
    let current_uri = current_document
        .or(event_document)
        .map(|document| document.uri().to_string());

    let Some(load_paths) = reload_plan.load_paths() else {
        debug!(
            current_document_uri = current_uri.as_deref(),
            open_document_count = open_documents.len(),
            reload_plan = ?reload_plan,
            "skipping workspace facts recomputation because no reload roots are available"
        );
        return WorkspaceLoadState::default();
    };
    let load_paths = load_paths.to_vec();
    let start = Instant::now();
    debug!(
        current_document_uri = current_uri.as_deref(),
        reload_plan = ?reload_plan,
        workspace_root_count = load_paths.len(),
        open_document_count = open_documents.len(),
        load_paths = ?load_paths,
        "recomputing workspace facts"
    );

    let mut loader = backend
        .workspace_loader()
        .lock()
        .expect("workspace loader mutex should not be poisoned");
    loader.clear_document_overrides();

    for open_document in &open_documents {
        add_document_override(&mut loader, open_document);
    }

    if let Some(current_document) = current_document {
        add_document_override(&mut loader, current_document);
    }

    match loader.load_paths_with_failures(load_paths) {
        Ok(workspace_facts) => {
            info!(
                current_document_uri = current_uri.as_deref(),
                document_count = workspace_facts.documents().len(),
                include_count = workspace_facts.includes().len(),
                workspace_instance_count = workspace_facts.workspace_indexes().len(),
                semantic_diagnostic_count = workspace_facts.semantic_diagnostics().len(),
                elapsed_ms = start.elapsed().as_millis(),
                "recomputed workspace facts"
            );
            WorkspaceLoadState {
                facts: Some(workspace_facts),
                failures: Vec::new(),
            }
        }
        Err(error) => {
            let failures = error.into_failures();
            warn!(
                current_document_uri = current_uri.as_deref(),
                elapsed_ms = start.elapsed().as_millis(),
                failure_count = failures.len(),
                failures = ?failures,
                "failed to recompute workspace facts"
            );
            WorkspaceLoadState {
                facts: None,
                failures,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkspaceReloadPlan {
    /// Re-scan configured workspace roots, or a single local anchor when no
    /// workspace roots exist. This is intentionally reserved for cases where
    /// the existing workspace facts cannot tell us which root files to reload.
    BroadScan {
        reason: BroadScanReason,
        roots: Vec<PathBuf>,
    },
    /// Reload the known workspace root files directly, letting the analysis
    /// loader reuse its document/session caches instead of walking directories.
    KnownRootFiles { roots: Vec<PathBuf> },
}

impl WorkspaceReloadPlan {
    fn new(
        workspace_roots: &[PathBuf],
        open_documents: &[DocumentState],
        workspace_facts: Option<&WorkspaceFacts>,
        workspace_load_failures: &[WorkspaceLoadFailure],
        event_document: Option<&DocumentState>,
    ) -> Self {
        let broad_scan = |reason| Self::BroadScan {
            reason,
            roots: broad_scan_roots(workspace_roots, open_documents, event_document),
        };

        if !workspace_load_failures.is_empty() {
            return broad_scan(BroadScanReason::FailureRecovery);
        }

        let Some(workspace_facts) = workspace_facts else {
            return broad_scan(BroadScanReason::ColdStart);
        };

        let Some(event_document_id) = event_document.and_then(workspace_document_id) else {
            return broad_scan(BroadScanReason::UnknownDocument);
        };

        if workspace_facts.document(&event_document_id).is_none() {
            return broad_scan(BroadScanReason::UnknownDocument);
        }

        let roots = known_workspace_root_files(workspace_facts);
        if roots.is_empty() {
            return broad_scan(BroadScanReason::UnknownDocument);
        }

        Self::KnownRootFiles { roots }
    }

    fn load_paths(&self) -> Option<&[PathBuf]> {
        match self {
            Self::BroadScan { roots, .. } | Self::KnownRootFiles { roots } => {
                (!roots.is_empty()).then_some(roots)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BroadScanReason {
    ColdStart,
    UnknownDocument,
    FailureRecovery,
}

fn broad_scan_roots(
    workspace_roots: &[PathBuf],
    open_documents: &[DocumentState],
    event_document: Option<&DocumentState>,
) -> Vec<PathBuf> {
    if !workspace_roots.is_empty() {
        return workspace_roots.to_vec();
    }

    // Without configured workspace roots, anchor discovery from the current
    // document first and then any other open document so include diagnostics
    // still have a local workspace to resolve against.
    event_document
        .and_then(canonical_document_path)
        .map(|path| vec![path])
        .or_else(|| {
            open_documents
                .iter()
                .find_map(canonical_document_path)
                .map(|path| vec![path])
        })
        .unwrap_or_default()
}

fn known_workspace_root_files(workspace_facts: &WorkspaceFacts) -> Vec<PathBuf> {
    workspace_facts
        .workspace_indexes()
        .iter()
        .map(|workspace_index| {
            workspace_document_path(workspace_facts, workspace_index.root_document())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn workspace_document_path(workspace_facts: &WorkspaceFacts, document_id: &DocumentId) -> PathBuf {
    workspace_facts
        .document(document_id)
        .and_then(|document| document.snapshot().location())
        .map_or_else(
            || PathBuf::from(document_id.as_str()),
            |location| location.path().to_path_buf(),
        )
}

fn snapshot_from_workspace_facts(
    document: &DocumentState,
    workspace_facts: Option<&WorkspaceFacts>,
) -> Option<strz_analysis::DocumentSnapshot> {
    let document_id = workspace_document_id(document)?;
    workspace_facts
        .and_then(|facts| facts.document(&document_id))
        .map(|workspace_document| workspace_document.snapshot().clone())
}

fn workspace_document_id(document: &DocumentState) -> Option<DocumentId> {
    document.workspace_document_id().cloned()
}

fn add_document_override(loader: &mut WorkspaceLoader, document: &DocumentState) {
    if let Some(path) = canonical_document_path(document) {
        loader.set_document_override(path, document.text().to_owned());
    }
}

fn canonical_document_path(document: &DocumentState) -> Option<PathBuf> {
    document.canonical_path().cloned()
}

fn canonical_file_path_from_uri(uri: &tower_lsp_server::ls_types::Uri) -> Option<PathBuf> {
    uri.to_file_path().map(std::borrow::Cow::into_owned)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use strz_analysis::WorkspaceLoader;
    use tower_lsp_server::ls_types::Uri;

    use super::{BroadScanReason, WorkspaceReloadPlan};
    use crate::documents::DocumentState;

    #[test]
    fn reload_plan_uses_cached_root_files_for_known_workspace_members() {
        let workspace_root = workspace_fixture_path("cross-file-navigation");
        let workspace_facts = workspace_facts_for_root(&workspace_root);
        let edited_document = document_state_for_path(&workspace_root.join("model.dsl"));

        let reload_plan = WorkspaceReloadPlan::new(
            std::slice::from_ref(&workspace_root),
            &[],
            Some(&workspace_facts),
            &[],
            Some(&edited_document),
        );

        assert_eq!(
            reload_plan,
            WorkspaceReloadPlan::KnownRootFiles {
                roots: vec![workspace_root.join("workspace.dsl").canonicalize().unwrap()]
            }
        );
    }

    #[test]
    fn reload_plan_broad_scans_before_workspace_facts_exist() {
        let workspace_root = workspace_fixture_path("cross-file-navigation");
        let edited_document = document_state_for_path(&workspace_root.join("model.dsl"));

        let reload_plan = WorkspaceReloadPlan::new(
            std::slice::from_ref(&workspace_root),
            &[],
            None,
            &[],
            Some(&edited_document),
        );

        assert_eq!(
            reload_plan,
            WorkspaceReloadPlan::BroadScan {
                reason: BroadScanReason::ColdStart,
                roots: vec![workspace_root],
            }
        );
    }

    #[test]
    fn reload_plan_broad_scans_unknown_workspace_members() {
        let workspace_root = workspace_fixture_path("cross-file-navigation");
        let unknown_workspace_root = workspace_fixture_path("minimal-scan");
        let workspace_facts = workspace_facts_for_root(&workspace_root);
        let edited_document =
            document_state_for_path(&unknown_workspace_root.join("workspace.dsl"));

        let reload_plan = WorkspaceReloadPlan::new(
            std::slice::from_ref(&workspace_root),
            &[],
            Some(&workspace_facts),
            &[],
            Some(&edited_document),
        );

        assert_eq!(
            reload_plan,
            WorkspaceReloadPlan::BroadScan {
                reason: BroadScanReason::UnknownDocument,
                roots: vec![workspace_root],
            }
        );
    }

    #[test]
    fn reload_plan_broad_scans_after_workspace_load_failures() {
        let workspace_root = workspace_fixture_path("cross-file-navigation");
        let workspace_facts = workspace_facts_for_root(&workspace_root);
        let edited_document = document_state_for_path(&workspace_root.join("model.dsl"));
        let failures = WorkspaceLoader::new()
            .load_paths_with_failures([workspace_root.join("missing")])
            .expect_err("missing root should produce structured load failures")
            .into_failures();

        let reload_plan = WorkspaceReloadPlan::new(
            std::slice::from_ref(&workspace_root),
            &[],
            Some(&workspace_facts),
            &failures,
            Some(&edited_document),
        );

        assert_eq!(
            reload_plan,
            WorkspaceReloadPlan::BroadScan {
                reason: BroadScanReason::FailureRecovery,
                roots: vec![workspace_root],
            }
        );
    }

    fn workspace_facts_for_root(root: &Path) -> strz_analysis::WorkspaceFacts {
        WorkspaceLoader::new()
            .load_paths_with_failures([root])
            .expect("workspace fixture should load cleanly")
    }

    fn document_state_for_path(path: &Path) -> DocumentState {
        let path = path.canonicalize().expect("fixture document should exist");
        DocumentState::new(
            Uri::from_file_path(&path).expect("fixture URI should parse"),
            1,
            fs::read_to_string(&path).expect("fixture document should be readable"),
        )
    }

    fn workspace_fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/lsp/workspaces")
            .join(name)
            .canonicalize()
            .expect("workspace fixture should exist")
    }
}
