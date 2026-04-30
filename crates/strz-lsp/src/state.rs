//! Shared server state that handlers can read without re-deriving protocol data.

use std::collections::{BTreeSet, HashMap};

use strz_analysis::{DocumentSnapshot, WorkspaceFacts, WorkspaceLoadFailure};
use tower_lsp_server::ls_types::{ClientCapabilities, Uri};

use crate::documents::DocumentStore;

/// Shared mutable state for the running language server session.
#[derive(Debug, Default)]
pub struct ServerState {
    client_capabilities: Option<ClientCapabilities>,
    workspace_roots: Vec<Uri>,
    documents: DocumentStore,
    snapshots: HashMap<Uri, DocumentSnapshot>,
    workspace_facts: Option<WorkspaceFacts>,
    /// Latest active workspace-load failures, kept in publication order.
    workspace_load_failures: Vec<WorkspaceLoadFailure>,
    /// Unanchored failures already surfaced to the client while they remain active.
    reported_unanchored_workspace_load_failures: BTreeSet<WorkspaceLoadFailure>,
}

impl ServerState {
    /// Stores the client capabilities reported during initialization.
    pub fn set_client_capabilities(&mut self, capabilities: ClientCapabilities) {
        self.client_capabilities = Some(capabilities);
    }

    /// Stores the workspace roots reported during initialization.
    pub fn set_workspace_roots(&mut self, workspace_roots: Vec<Uri>) {
        self.workspace_roots = workspace_roots;
    }

    /// Returns the workspace roots reported during initialization.
    #[must_use]
    pub fn workspace_roots(&self) -> &[Uri] {
        &self.workspace_roots
    }

    /// Returns the open-document store.
    #[must_use]
    pub const fn documents(&self) -> &DocumentStore {
        &self.documents
    }

    /// Returns the open-document store mutably.
    #[must_use]
    pub const fn documents_mut(&mut self) -> &mut DocumentStore {
        &mut self.documents
    }

    /// Associates an analyzed snapshot with a document URI.
    pub fn set_snapshot(&mut self, uri: Uri, snapshot: DocumentSnapshot) {
        self.snapshots.insert(uri, snapshot);
    }

    /// Looks up the latest analyzed snapshot for a document URI.
    #[must_use]
    pub fn snapshot(&self, uri: &Uri) -> Option<&DocumentSnapshot> {
        self.snapshots.get(uri)
    }

    /// Removes the cached analyzed snapshot for a document URI.
    pub fn remove_snapshot(&mut self, uri: &Uri) {
        self.snapshots.remove(uri);
    }

    /// Replaces the cached workspace discovery facts for the current session.
    pub fn set_workspace_facts(&mut self, workspace_facts: Option<WorkspaceFacts>) {
        self.workspace_facts = workspace_facts;
    }

    /// Returns the latest cached workspace discovery facts, if any.
    #[must_use]
    pub const fn workspace_facts(&self) -> Option<&WorkspaceFacts> {
        self.workspace_facts.as_ref()
    }

    /// Replaces the latest structured workspace-load failures.
    ///
    /// Returns unanchored messages that have not already been surfaced to the
    /// user while the same failure remains active.
    pub fn set_workspace_load_failures(
        &mut self,
        failures: Vec<WorkspaceLoadFailure>,
    ) -> Vec<String> {
        let has_unanchored_failures = failures.iter().any(|failure| !failure.is_anchored());
        if !has_unanchored_failures {
            self.reported_unanchored_workspace_load_failures.clear();
        }

        let new_unanchored_messages = failures
            .iter()
            .filter(|failure| !failure.is_anchored())
            .filter(|failure| {
                self.reported_unanchored_workspace_load_failures
                    .insert((*failure).clone())
            })
            .map(|failure| failure.message().to_owned())
            .collect();

        self.workspace_load_failures = failures;
        new_unanchored_messages
    }

    /// Returns the latest structured workspace-load failures.
    #[must_use]
    pub fn workspace_load_failures(&self) -> &[WorkspaceLoadFailure] {
        &self.workspace_load_failures
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    use strz_analysis::WorkspaceLoader;

    use super::ServerState;

    #[test]
    fn unanchored_failures_are_re_reported_after_a_clear() {
        let missing_root = missing_root_path("clear-and-rereport");
        let failures = workspace_root_failures(&missing_root);
        let message = failures[0].message().to_owned();
        let mut state = ServerState::default();

        assert_eq!(
            state.set_workspace_load_failures(failures.clone()),
            vec![message.clone()]
        );
        assert!(
            state
                .set_workspace_load_failures(failures.clone())
                .is_empty()
        );

        state.set_workspace_load_failures(Vec::new());

        assert_eq!(state.set_workspace_load_failures(failures), vec![message]);
    }

    #[test]
    fn reported_unanchored_failures_keep_structured_identity() {
        let missing_root = missing_root_path("structured-key");
        let failures = workspace_root_failures(&missing_root);
        let mut state = ServerState::default();

        state.set_workspace_load_failures(failures.clone());

        let reported = state
            .reported_unanchored_workspace_load_failures
            .iter()
            .next()
            .expect("state should keep one structured unanchored failure");
        assert_eq!(reported, &failures[0]);
    }

    fn workspace_root_failures(path: &Path) -> Vec<strz_analysis::WorkspaceLoadFailure> {
        let mut loader = WorkspaceLoader::new();
        loader
            .load_paths_with_failures([path])
            .expect_err("missing workspace root should produce an unanchored failure")
            .into_failures()
    }

    fn missing_root_path(suffix: &str) -> PathBuf {
        let path = env::current_dir()
            .expect("tests should run from a readable current directory")
            .join(format!(
                "tmp/server-state-missing-root-{suffix}-{}",
                std::process::id()
            ));
        assert!(
            !path.exists(),
            "test missing-root path should not already exist: {}",
            path.display()
        );
        path
    }
}
