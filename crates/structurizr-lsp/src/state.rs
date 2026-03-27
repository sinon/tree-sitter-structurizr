//! Shared server state that handlers can read without re-deriving protocol data.

use std::collections::HashMap;

use structurizr_analysis::DocumentSnapshot;
use tower_lsp_server::ls_types::{ClientCapabilities, Uri};

use crate::documents::DocumentStore;

/// Shared mutable state for the running language server session.
#[derive(Debug, Default)]
pub struct ServerState {
    client_capabilities: Option<ClientCapabilities>,
    workspace_roots: Vec<Uri>,
    documents: DocumentStore,
    snapshots: HashMap<Uri, DocumentSnapshot>,
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

    /// Returns the open-document store.
    #[must_use]
    pub const fn documents(&self) -> &DocumentStore {
        &self.documents
    }

    /// Returns the open-document store mutably.
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
}
