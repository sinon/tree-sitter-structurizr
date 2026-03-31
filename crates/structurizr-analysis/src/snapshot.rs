//! Immutable document inputs and snapshots used as the crate's main exchange objects.

use std::path::{Path, PathBuf};

use tree_sitter::Tree;

use crate::constants::ConstantDefinition;
use crate::diagnostics::SyntaxDiagnostic;
use crate::extract;
use crate::includes::IncludeDirective;
use crate::symbols::{IdentifierModeFact, Reference, Symbol};

/// Stable caller-provided identifier for a document across analysis runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(String);

impl DocumentId {
    #[must_use]
    /// Creates a document identifier from any owned or borrowed string input.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    /// Returns the identifier as a borrowed string slice.
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

/// Filesystem location metadata attached to a document when available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLocation {
    path: PathBuf,
}

impl DocumentLocation {
    #[must_use]
    /// Creates a document location from a filesystem path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    /// Returns the path backing this location.
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

/// Input required to analyze one Structurizr document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInput {
    id: DocumentId,
    location: Option<DocumentLocation>,
    source: String,
}

impl DocumentInput {
    #[must_use]
    /// Creates a document input from a stable identifier and source text.
    pub fn new(id: impl Into<DocumentId>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            location: None,
            source: source.into(),
        }
    }

    #[must_use]
    /// Attaches filesystem location metadata to this input.
    pub fn with_location(mut self, location: impl Into<DocumentLocation>) -> Self {
        self.location = Some(location.into());
        self
    }

    #[must_use]
    /// Returns the caller-provided document identifier.
    pub const fn id(&self) -> &DocumentId {
        &self.id
    }

    #[must_use]
    /// Returns the optional filesystem location for this input.
    pub const fn location(&self) -> Option<&DocumentLocation> {
        self.location.as_ref()
    }

    #[must_use]
    /// Returns the full source text that will be analyzed.
    pub fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn into_parts(self) -> (DocumentId, Option<DocumentLocation>, String) {
        (self.id, self.location, self.source)
    }
}

/// Stable syntax-level facts extracted from one analyzed document.
///
/// This is the Salsa-friendly boundary for document analysis: everything here is
/// derived from one document's source text and is reusable without needing to
/// expose the Tree-sitter parse tree itself as the main cache boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSyntaxFacts {
    is_workspace_entry: bool,
    syntax_diagnostics: Vec<SyntaxDiagnostic>,
    include_directives: Vec<IncludeDirective>,
    constant_definitions: Vec<ConstantDefinition>,
    identifier_modes: Vec<IdentifierModeFact>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
}

impl DocumentSyntaxFacts {
    /// Extracts the stable syntax-level facts from one parsed document.
    pub(crate) fn collect(tree: &Tree, source: &str) -> Self {
        let syntax_diagnostics = extract::diagnostics::collect(tree);
        let include_directives = extract::includes::collect(tree, source);
        let constant_definitions = extract::constants::collect(tree, source);
        let identifier_modes = extract::symbols::collect_identifier_modes(tree, source);
        let (symbols, references) = extract::symbols::collect_symbols_and_references(tree, source);

        Self {
            is_workspace_entry: contains_workspace_entry(tree),
            syntax_diagnostics,
            include_directives,
            constant_definitions,
            identifier_modes,
            symbols,
            references,
        }
    }

    /// Returns whether the document contains a top-level `workspace` block.
    #[must_use]
    pub const fn is_workspace_entry(&self) -> bool {
        self.is_workspace_entry
    }

    /// Returns whether any syntax diagnostics were extracted from the parse tree.
    #[must_use]
    pub const fn has_syntax_errors(&self) -> bool {
        !self.syntax_diagnostics.is_empty()
    }

    /// Returns all syntax diagnostics found while traversing the parse tree.
    #[must_use]
    pub fn syntax_diagnostics(&self) -> &[SyntaxDiagnostic] {
        &self.syntax_diagnostics
    }

    /// Returns all raw `!include` directives found in the document.
    #[must_use]
    pub fn include_directives(&self) -> &[IncludeDirective] {
        &self.include_directives
    }

    /// Returns all ordered string-constant definitions extracted from the document.
    #[must_use]
    pub fn constant_definitions(&self) -> &[ConstantDefinition] {
        &self.constant_definitions
    }

    /// Returns all extracted `!identifiers` mode directives in the document.
    #[must_use]
    pub fn identifier_modes(&self) -> &[IdentifierModeFact] {
        &self.identifier_modes
    }

    /// Returns all declaration symbols extracted from the document.
    #[must_use]
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Returns all symbol references extracted from the document.
    #[must_use]
    pub fn references(&self) -> &[Reference] {
        &self.references
    }
}

/// Private parsed-document payload cached behind the public snapshot facade.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    tree: Tree,
    syntax_facts: DocumentSyntaxFacts,
}

impl ParsedDocument {
    pub(crate) const fn new(tree: Tree, syntax_facts: DocumentSyntaxFacts) -> Self {
        Self { tree, syntax_facts }
    }

    /// Clones the cached parsed result into the public snapshot shape expected by
    /// current callers.
    pub(crate) fn to_snapshot(&self, input: DocumentInput) -> DocumentSnapshot {
        let (id, location, source) = input.into_parts();

        DocumentSnapshot {
            id,
            location,
            source,
            tree: self.tree.clone(),
            syntax_facts: self.syntax_facts.clone(),
        }
    }
}

/// Immutable snapshot produced by analyzing one Structurizr document.
///
/// A snapshot groups the original source, parse tree, and extracted facts so
/// downstream tooling can answer syntax and navigation queries from one shared
/// object.
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    id: DocumentId,
    location: Option<DocumentLocation>,
    source: String,
    tree: Tree,
    syntax_facts: DocumentSyntaxFacts,
}

impl DocumentSnapshot {
    #[must_use]
    /// Returns the document identifier carried through analysis.
    pub const fn id(&self) -> &DocumentId {
        &self.id
    }

    #[must_use]
    /// Returns the optional filesystem location supplied with the input.
    pub const fn location(&self) -> Option<&DocumentLocation> {
        self.location.as_ref()
    }

    #[must_use]
    /// Returns the exact source text that produced this snapshot.
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    /// Returns the Tree-sitter parse tree for the analyzed source.
    pub const fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Returns the stable syntax-level facts extracted from the document.
    #[must_use]
    pub const fn syntax_facts(&self) -> &DocumentSyntaxFacts {
        &self.syntax_facts
    }

    #[must_use]
    /// Returns whether the document contains a top-level `workspace` block.
    pub const fn is_workspace_entry(&self) -> bool {
        self.syntax_facts.is_workspace_entry()
    }

    #[must_use]
    /// Returns whether any syntax diagnostics were extracted from the parse tree.
    pub const fn has_syntax_errors(&self) -> bool {
        self.syntax_facts.has_syntax_errors()
    }

    #[must_use]
    /// Returns all syntax diagnostics found while traversing the parse tree.
    pub fn syntax_diagnostics(&self) -> &[SyntaxDiagnostic] {
        self.syntax_facts.syntax_diagnostics()
    }

    #[must_use]
    /// Returns all raw `!include` directives found in the document.
    pub fn include_directives(&self) -> &[IncludeDirective] {
        self.syntax_facts.include_directives()
    }

    #[must_use]
    /// Returns all ordered string-constant definitions extracted from the document.
    pub fn constant_definitions(&self) -> &[ConstantDefinition] {
        self.syntax_facts.constant_definitions()
    }

    #[must_use]
    /// Returns all extracted `!identifiers` mode directives in the document.
    pub fn identifier_modes(&self) -> &[IdentifierModeFact] {
        self.syntax_facts.identifier_modes()
    }

    #[must_use]
    /// Returns all declaration symbols extracted from the document.
    pub fn symbols(&self) -> &[Symbol] {
        self.syntax_facts.symbols()
    }

    #[must_use]
    /// Returns all symbol references extracted from the document.
    pub fn references(&self) -> &[Reference] {
        self.syntax_facts.references()
    }
}

fn contains_workspace_entry(tree: &Tree) -> bool {
    let root = tree.root_node();
    let mut cursor = root.walk();

    root.named_children(&mut cursor)
        .any(|child| matches!(child.kind(), "workspace" | "workspace_block"))
}
