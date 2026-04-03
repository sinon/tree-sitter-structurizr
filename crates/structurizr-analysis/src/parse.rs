//! Parser orchestration and the snapshot-producing analysis entrypoints.

use std::collections::{HashMap, hash_map::Entry};
use std::sync::Mutex;

use salsa::Setter as _;
use tree_sitter::{Parser, Tree};

use crate::snapshot::{DocumentInput, DocumentSnapshot, DocumentSyntaxFacts, ParsedDocument};

// =============================================================================
// Salsa-backed parsed-document cache
// =============================================================================
//
// The first Salsa integration stays behind `DocumentAnalyzer` so downstream
// crates can keep using `DocumentSnapshot` unchanged. We memoize one parsed
// document per stable caller-provided document id and source revision, while
// still returning an owned snapshot that includes a Tree-sitter tree.

#[salsa::input]
struct IncrementalDocument {
    #[returns(ref)]
    source: String,
}

#[salsa::db]
trait IncrementalAnalysisDb: salsa::Database {
    fn parser(&self) -> &Mutex<Parser>;
}

#[salsa::tracked(returns(ref), no_eq, unsafe(non_update_types))]
fn parsed_document(
    db: &dyn IncrementalAnalysisDb,
    document: IncrementalDocument,
) -> ParsedDocument {
    let source = document.source(db);
    let tree = parse_source(db, source);
    let syntax_facts = DocumentSyntaxFacts::collect(&tree, source);

    ParsedDocument::new(tree, syntax_facts)
}

#[salsa::db]
struct IncrementalAnalysisDatabase {
    storage: salsa::Storage<Self>,
    parser: Mutex<Parser>,
    #[cfg(test)]
    logs: std::sync::Arc<std::sync::Mutex<Option<Vec<String>>>>,
}

#[salsa::db]
impl salsa::Database for IncrementalAnalysisDatabase {}

#[salsa::db]
impl IncrementalAnalysisDb for IncrementalAnalysisDatabase {
    fn parser(&self) -> &Mutex<Parser> {
        &self.parser
    }
}

impl Default for IncrementalAnalysisDatabase {
    fn default() -> Self {
        #[cfg(test)]
        {
            let logs: std::sync::Arc<std::sync::Mutex<Option<Vec<String>>>> =
                std::sync::Arc::new(std::sync::Mutex::new(None));

            Self {
                storage: salsa::Storage::new(Some(Box::new({
                    let logs = logs.clone();
                    move |event| {
                        if let salsa::EventKind::WillExecute { .. } = event.kind
                            && let Some(logs) =
                                &mut *logs.lock().expect("Salsa log mutex should not be poisoned")
                        {
                            logs.push(format!("{event:?}"));
                        }
                    }
                }))),
                parser: Mutex::new(structurizr_parser()),
                logs,
            }
        }

        #[cfg(not(test))]
        {
            Self {
                storage: salsa::Storage::new(None),
                parser: Mutex::new(structurizr_parser()),
            }
        }
    }
}

#[cfg(test)]
impl IncrementalAnalysisDatabase {
    fn enable_logging(&self) {
        let mut logs = self
            .logs
            .lock()
            .expect("Salsa log mutex should not be poisoned");
        if logs.is_none() {
            *logs = Some(Vec::new());
        }
    }

    fn take_logs(&self) -> Vec<String> {
        let mut logs = self
            .logs
            .lock()
            .expect("Salsa log mutex should not be poisoned");
        logs.as_mut().map_or_else(Vec::new, std::mem::take)
    }
}

// =============================================================================
// Public analyzer API
// =============================================================================

/// Reusable parser-backed entrypoint for analyzing Structurizr documents.
pub struct DocumentAnalyzer {
    db: IncrementalAnalysisDatabase,
    documents: HashMap<crate::DocumentId, CachedDocument>,
}

struct CachedDocument {
    tracked: IncrementalDocument,
    source: String,
}

impl DocumentAnalyzer {
    /// Creates a parser-backed analyzer for repeated Structurizr document analysis.
    ///
    /// # Panics
    ///
    /// Panics if the checked-in Structurizr Tree-sitter language cannot be loaded.
    #[must_use]
    pub fn new() -> Self {
        Self {
            db: IncrementalAnalysisDatabase::default(),
            documents: HashMap::new(),
        }
    }

    /// Parses one document and returns an immutable snapshot of extracted facts.
    ///
    /// The resulting snapshot keeps the original source, parse tree, syntax
    /// diagnostics, include directives, constant definitions, identifier-mode
    /// directives, symbols, and references together so downstream tools can
    /// answer queries without re-parsing immediately.
    #[must_use]
    pub fn analyze(&mut self, input: DocumentInput) -> DocumentSnapshot {
        let document = self.tracked_document(input.id(), input.source());
        let parsed = parsed_document(&self.db, document);

        parsed.to_snapshot(input)
    }

    fn tracked_document(&mut self, id: &crate::DocumentId, source: &str) -> IncrementalDocument {
        match self.documents.entry(id.clone()) {
            Entry::Occupied(entry) => {
                let cached = entry.into_mut();
                if cached.source != source {
                    let updated_source = source.to_owned();
                    cached
                        .tracked
                        .set_source(&mut self.db)
                        .to(updated_source.clone());
                    cached.source = updated_source;
                }
                cached.tracked
            }
            Entry::Vacant(entry) => {
                let source = source.to_owned();
                let tracked = IncrementalDocument::new(&self.db, source.clone());
                entry.insert(CachedDocument { tracked, source });
                tracked
            }
        }
    }
}

impl Default for DocumentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

fn structurizr_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_structurizr::LANGUAGE.into())
        .expect("Structurizr language should load");
    parser
}

/// Parses source text into a syntax tree for one analysis run.
///
/// # Panics
///
/// Panics if Tree-sitter fails to produce a tree, which would indicate a
/// parser invariant violation rather than invalid user input.
fn parse_source(db: &dyn IncrementalAnalysisDb, source: &str) -> Tree {
    db.parser()
        .lock()
        .expect("Structurizr parser mutex should not be poisoned")
        .parse(source, None)
        .expect("Parser should return a tree")
}

#[cfg(test)]
mod tests {
    use super::DocumentAnalyzer;
    use crate::snapshot::DocumentInput;

    fn parse_execution_logs(logs: &[String]) -> usize {
        logs.iter()
            .filter(|log| log.contains("parsed_document"))
            .count()
    }

    #[test]
    fn repeated_analysis_of_same_document_reuses_cached_query_result() {
        let mut analyzer = DocumentAnalyzer::new();
        analyzer.db.enable_logging();

        let input = DocumentInput::new(
            "workspace.dsl",
            "workspace { model { user = person \"User\" } }",
        );

        let first = analyzer.analyze(input.clone());
        let second = analyzer.analyze(input);

        assert_eq!(first.symbols(), second.symbols());
        assert_eq!(parse_execution_logs(&analyzer.db.take_logs()), 1);
    }

    #[test]
    fn source_changes_reexecute_cached_query() {
        let mut analyzer = DocumentAnalyzer::new();
        analyzer.db.enable_logging();

        let _first = analyzer.analyze(DocumentInput::new(
            "workspace.dsl",
            "workspace { model { user = person \"User\" } }",
        ));
        let _ = analyzer.db.take_logs();

        let second = analyzer.analyze(DocumentInput::new(
            "workspace.dsl",
            "workspace { model { admin = person \"Admin\" } }",
        ));

        assert_eq!(second.symbols().len(), 1);
        assert_eq!(parse_execution_logs(&analyzer.db.take_logs()), 1);
    }
}
