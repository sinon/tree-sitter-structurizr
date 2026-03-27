//! Immutable document inputs and snapshots used as the crate's main exchange objects.

use std::path::{Path, PathBuf};

use tree_sitter::Tree;

use crate::diagnostics::SyntaxDiagnostic;
use crate::includes::IncludeDirective;
use crate::symbols::{IdentifierModeFact, Reference, Symbol};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(String);

impl DocumentId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DocumentId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DocumentId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLocation {
    path: PathBuf,
}

impl DocumentLocation {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl From<PathBuf> for DocumentLocation {
    fn from(path: PathBuf) -> Self {
        Self::new(path)
    }
}

impl From<&Path> for DocumentLocation {
    fn from(path: &Path) -> Self {
        Self::new(path.to_path_buf())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInput {
    id: DocumentId,
    location: Option<DocumentLocation>,
    source: String,
}

impl DocumentInput {
    #[must_use]
    pub fn new(id: impl Into<DocumentId>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            location: None,
            source: source.into(),
        }
    }

    #[must_use]
    pub fn with_location(mut self, location: impl Into<DocumentLocation>) -> Self {
        self.location = Some(location.into());
        self
    }

    #[must_use]
    pub const fn id(&self) -> &DocumentId {
        &self.id
    }

    #[must_use]
    pub const fn location(&self) -> Option<&DocumentLocation> {
        self.location.as_ref()
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn into_parts(self) -> (DocumentId, Option<DocumentLocation>, String) {
        (self.id, self.location, self.source)
    }
}

#[derive(Debug)]
pub struct DocumentSnapshot {
    id: DocumentId,
    location: Option<DocumentLocation>,
    source: String,
    tree: Tree,
    syntax_diagnostics: Vec<SyntaxDiagnostic>,
    include_directives: Vec<IncludeDirective>,
    identifier_modes: Vec<IdentifierModeFact>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
}

impl DocumentSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        id: DocumentId,
        location: Option<DocumentLocation>,
        source: String,
        tree: Tree,
        syntax_diagnostics: Vec<SyntaxDiagnostic>,
        include_directives: Vec<IncludeDirective>,
        identifier_modes: Vec<IdentifierModeFact>,
        symbols: Vec<Symbol>,
        references: Vec<Reference>,
    ) -> Self {
        Self {
            id,
            location,
            source,
            tree,
            syntax_diagnostics,
            include_directives,
            identifier_modes,
            symbols,
            references,
        }
    }

    #[must_use]
    pub const fn id(&self) -> &DocumentId {
        &self.id
    }

    #[must_use]
    pub const fn location(&self) -> Option<&DocumentLocation> {
        self.location.as_ref()
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub const fn tree(&self) -> &Tree {
        &self.tree
    }

    #[must_use]
    pub const fn has_syntax_errors(&self) -> bool {
        !self.syntax_diagnostics.is_empty()
    }

    #[must_use]
    pub fn syntax_diagnostics(&self) -> &[SyntaxDiagnostic] {
        &self.syntax_diagnostics
    }

    #[must_use]
    pub fn include_directives(&self) -> &[IncludeDirective] {
        &self.include_directives
    }

    #[must_use]
    pub fn identifier_modes(&self) -> &[IdentifierModeFact] {
        &self.identifier_modes
    }

    #[must_use]
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    #[must_use]
    pub fn references(&self) -> &[Reference] {
        &self.references
    }
}
