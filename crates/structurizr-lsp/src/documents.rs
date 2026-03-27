//! Open-document storage kept separate from protocol handlers.

use std::collections::HashMap;

use line_index::LineIndex;
use structurizr_analysis::DocumentInput;
use tower_lsp_server::ls_types::Uri;

/// Tracks the latest open-text state for one LSP document.
#[derive(Debug, Clone)]
pub struct DocumentState {
    uri: Uri,
    version: i32,
    text: String,
    line_index: LineIndex,
}

impl DocumentState {
    /// Creates an open-document state from the latest URI, version, and text.
    #[must_use]
    pub fn new(uri: Uri, version: i32, text: String) -> Self {
        let line_index = LineIndex::new(&text);

        Self {
            uri,
            version,
            text,
            line_index,
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

        if let Some(path) = self.uri.to_file_path() {
            input.with_location(path.into_owned())
        } else {
            input
        }
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
}
