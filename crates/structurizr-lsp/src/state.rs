//! Shared server state that handlers can read without re-deriving protocol data.

use std::collections::HashMap;

use structurizr_analysis::DocumentSnapshot;
use tower_lsp_server::ls_types::{ClientCapabilities, Uri};

use crate::documents::DocumentStore;

#[derive(Debug, Default)]
pub struct ServerState {
    client_capabilities: Option<ClientCapabilities>,
    workspace_roots: Vec<Uri>,
    documents: DocumentStore,
    snapshots: HashMap<Uri, DocumentSnapshot>,
}

impl ServerState {
    pub fn set_client_capabilities(&mut self, capabilities: ClientCapabilities) {
        self.client_capabilities = Some(capabilities);
    }

    pub fn set_workspace_roots(&mut self, workspace_roots: Vec<Uri>) {
        self.workspace_roots = workspace_roots;
    }

    #[must_use]
    pub fn documents(&self) -> &DocumentStore {
        &self.documents
    }

    pub fn documents_mut(&mut self) -> &mut DocumentStore {
        &mut self.documents
    }

    pub fn set_snapshot(&mut self, uri: Uri, snapshot: DocumentSnapshot) {
        self.snapshots.insert(uri, snapshot);
    }

    #[must_use]
    pub fn snapshot(&self, uri: &Uri) -> Option<&DocumentSnapshot> {
        self.snapshots.get(uri)
    }

    pub fn remove_snapshot(&mut self, uri: &Uri) {
        self.snapshots.remove(uri);
    }
}
