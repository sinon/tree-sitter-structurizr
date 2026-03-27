//! Open-document storage kept separate from protocol handlers.

use std::collections::HashMap;

use line_index::LineIndex;
use structurizr_analysis::DocumentInput;
use tower_lsp_server::ls_types::Uri;

#[derive(Debug, Clone)]
pub struct DocumentState {
    uri: Uri,
    version: i32,
    text: String,
    line_index: LineIndex,
}

impl DocumentState {
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

    #[must_use]
    pub const fn uri(&self) -> &Uri {
        &self.uri
    }

    #[must_use]
    pub const fn version(&self) -> i32 {
        self.version
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn line_index(&self) -> &LineIndex {
        &self.line_index
    }

    pub fn replace_text(&mut self, version: i32, text: String) {
        self.version = version;
        self.line_index = LineIndex::new(&text);
        self.text = text;
    }

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

#[derive(Debug, Default)]
pub struct DocumentStore {
    open_documents: HashMap<Uri, DocumentState>,
}

impl DocumentStore {
    pub fn open(&mut self, document: DocumentState) {
        self.open_documents.insert(document.uri().clone(), document);
    }

    pub fn close(&mut self, uri: &Uri) {
        self.open_documents.remove(uri);
    }

    #[must_use]
    pub fn get(&self, uri: &Uri) -> Option<&DocumentState> {
        self.open_documents.get(uri)
    }

    pub fn get_mut(&mut self, uri: &Uri) -> Option<&mut DocumentState> {
        self.open_documents.get_mut(uri)
    }
}
