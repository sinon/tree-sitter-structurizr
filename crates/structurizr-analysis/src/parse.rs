//! Parser orchestration and the snapshot-producing analysis entrypoints.

use tree_sitter::{Parser, Tree};

use crate::extract;
use crate::snapshot::{DocumentInput, DocumentSnapshot};

/// Reusable parser-backed entrypoint for analyzing Structurizr documents.
pub struct DocumentAnalyzer {
    parser: Parser,
}

impl DocumentAnalyzer {
    /// Creates a parser-backed analyzer for repeated Structurizr document analysis.
    ///
    /// # Panics
    ///
    /// Panics if the checked-in Structurizr Tree-sitter language cannot be loaded.
    #[must_use]
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_structurizr::LANGUAGE.into())
            .expect("Structurizr language should load");

        Self { parser }
    }

    /// Parses one document and returns an immutable snapshot of extracted facts.
    ///
    /// The resulting snapshot keeps the original source, parse tree, syntax
    /// diagnostics, include directives, constant definitions, identifier-mode
    /// directives, symbols, and references together so downstream tools can
    /// answer queries without re-parsing immediately.
    #[must_use]
    pub fn analyze(&mut self, input: DocumentInput) -> DocumentSnapshot {
        let (id, location, source) = input.into_parts();
        let tree = self.parse(&source);
        let syntax_diagnostics = extract::diagnostics::collect(&tree);
        let include_directives = extract::includes::collect(&tree, &source);
        let constant_definitions = extract::constants::collect(&tree, &source);
        let identifier_modes = extract::symbols::collect_identifier_modes(&tree, &source);
        let (symbols, references) =
            extract::symbols::collect_symbols_and_references(&tree, &source);

        DocumentSnapshot::new(
            id,
            location,
            source,
            tree,
            syntax_diagnostics,
            include_directives,
            constant_definitions,
            identifier_modes,
            symbols,
            references,
        )
    }

    /// Parses source text into a syntax tree for one analysis run.
    ///
    /// # Panics
    ///
    /// Panics if Tree-sitter fails to produce a tree, which would indicate a
    /// parser invariant violation rather than invalid user input.
    fn parse(&mut self, source: &str) -> Tree {
        self.parser
            .parse(source, None)
            .expect("Parser should return a tree")
    }
}

impl Default for DocumentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
/// Convenience helper for analyzing a single document with a fresh parser.
pub fn analyze_document(input: DocumentInput) -> DocumentSnapshot {
    DocumentAnalyzer::new().analyze(input)
}
