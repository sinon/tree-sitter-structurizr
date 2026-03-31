//! Open-document storage kept separate from protocol handlers.

use std::{collections::HashMap, fs, path::PathBuf};

use line_index::LineIndex;
use structurizr_analysis::{DocumentId, DocumentInput};
use tower_lsp_server::ls_types::Uri;

/// Tracks the latest open-text state for one LSP document.
#[derive(Debug, Clone)]
pub struct DocumentState {
    uri: Uri,
    version: i32,
    text: String,
    line_index: LineIndex,
    canonical_path: Option<PathBuf>,
    workspace_document_id: Option<DocumentId>,
}

impl DocumentState {
    /// Creates an open-document state from the latest URI, version, and text.
    #[must_use]
    pub fn new(uri: Uri, version: i32, text: String) -> Self {
        let line_index = LineIndex::new(&text);
        let canonical_path = canonical_file_path_from_uri(&uri);
        let workspace_document_id = canonical_path
            .as_ref()
            .map(|path| DocumentId::new(path.to_string_lossy().into_owned()));

        Self {
            uri,
            version,
            text,
            line_index,
            canonical_path,
            workspace_document_id,
        }
    }

    /// Returns the document URI.
    #[must_use]
    pub const fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Returns the most recent protocol version for this document.
    #[must_use]
    pub const fn version(&self) -> i32 {
        self.version
    }

    /// Returns the latest full text for this document.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the cached line index for protocol position conversions.
    #[must_use]
    pub const fn line_index(&self) -> &LineIndex {
        &self.line_index
    }

    /// Returns the canonical filesystem path for this document when it is file-backed.
    #[must_use]
    pub const fn canonical_path(&self) -> Option<&PathBuf> {
        self.canonical_path.as_ref()
    }

    /// Returns the canonical workspace document identity when one exists.
    #[must_use]
    pub const fn workspace_document_id(&self) -> Option<&DocumentId> {
        self.workspace_document_id.as_ref()
    }

    /// Replaces the current text and recomputes the cached line index.
    pub fn replace_text(&mut self, version: i32, text: String) {
        self.version = version;
        self.line_index = LineIndex::new(&text);
        self.text = text;
    }

    /// Converts the open document into an analysis input snapshot request.
    ///
    /// When the URI maps to a local filesystem path, that location metadata is
    /// attached to the returned analysis input.
    #[must_use]
    pub fn to_input(&self) -> DocumentInput {
        let input = DocumentInput::new(self.uri.as_str().to_owned(), self.text.clone());

        if let Some(path) = self.canonical_path() {
            input.with_location(path.clone())
        } else {
            input
        }
    }
}

fn canonical_file_path_from_uri(uri: &Uri) -> Option<PathBuf> {
    let path = uri.to_file_path()?;
    fs::canonicalize(&path).ok()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tower_lsp_server::ls_types::Uri;

    use super::DocumentState;

    #[test]
    fn file_backed_documents_cache_canonical_identity() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/lsp/workspaces/minimal-scan/workspace.dsl");
        let canonical_path =
            std::fs::canonicalize(&path).expect("fixture workspace path should canonicalize");
        let uri = Uri::from_file_path(&path).expect("fixture URI should parse");
        let document = DocumentState::new(uri, 1, "workspace {}".to_owned());

        assert_eq!(document.canonical_path(), Some(&canonical_path));
        assert_eq!(
            document
                .workspace_document_id()
                .expect("file-backed document should cache a document id")
                .as_str(),
            canonical_path.to_string_lossy()
        );
        assert_eq!(
            document
                .to_input()
                .location()
                .expect("file-backed document input should carry a location")
                .path(),
            canonical_path.as_path()
        );
    }

    #[test]
    fn non_file_documents_do_not_cache_file_identity() {
        let uri = "untitled:structurizr"
            .parse::<Uri>()
            .expect("non-file URI should parse");
        let document = DocumentState::new(uri, 1, "workspace {}".to_owned());

        assert!(document.canonical_path().is_none());
        assert!(document.workspace_document_id().is_none());
        assert!(document.to_input().location().is_none());
    }
}

/// Stores the set of documents currently open in the language server.
#[derive(Debug, Default)]
pub struct DocumentStore {
    open_documents: HashMap<Uri, DocumentState>,
}

impl DocumentStore {
    /// Inserts or replaces an open document state.
    pub fn open(&mut self, document: DocumentState) {
        self.open_documents.insert(document.uri().clone(), document);
    }

    /// Removes an open document by URI.
    pub fn close(&mut self, uri: &Uri) {
        self.open_documents.remove(uri);
    }

    /// Looks up an open document by URI.
    #[must_use]
    pub fn get(&self, uri: &Uri) -> Option<&DocumentState> {
        self.open_documents.get(uri)
    }

    /// Looks up an open document mutably by URI.
    pub fn get_mut(&mut self, uri: &Uri) -> Option<&mut DocumentState> {
        self.open_documents.get_mut(uri)
    }

    /// Iterates over the currently open documents.
    pub fn iter(&self) -> impl Iterator<Item = &DocumentState> {
        self.open_documents.values()
    }

    /// Returns how many documents are currently open.
    #[must_use]
    pub fn len(&self) -> usize {
        self.open_documents.len()
    }

    /// Returns whether there are no currently open documents.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.open_documents.is_empty()
    }
}
