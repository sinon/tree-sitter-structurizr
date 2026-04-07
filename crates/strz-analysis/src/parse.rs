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
    use indoc::indoc;

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

    #[test]
    fn analysis_extracts_hover_metadata_from_supported_symbol_declarations() {
        let mut analyzer = DocumentAnalyzer::new();
        let snapshot = analyzer.analyze(DocumentInput::new(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User" "Header user description" "External, Browser" {
                            description "Body user description"
                            tag "Customer"
                            url "https://example.com/user"
                        }

                        system = softwareSystem "Payments Platform" {
                            api = container "Payments API" "Processes payment requests" "Rust" "Internal, HTTP" {
                                technology "Axum"
                                tags "Internal, Edge"
                                url "https://example.com/api"
                            }
                        }

                        rel = user -> api "Uses" "HTTPS" "Sync, Critical" {
                            description "Body relationship description"
                            technology "Mutual TLS"
                            tag "Observed"
                            url "https://example.com/rel"
                        }
                    }
                }
            "#},
        ));

        let user = snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.binding_name.as_deref() == Some("user"))
            .expect("user symbol should exist");
        assert_eq!(user.display_name, "User");
        assert_eq!(user.description.as_deref(), Some("Body user description"));
        assert_eq!(user.technology, None);
        assert_eq!(user.tags, vec!["External", "Browser", "Customer"]);
        assert_eq!(user.url.as_deref(), Some("https://example.com/user"));

        let api = snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.binding_name.as_deref() == Some("api"))
            .expect("api symbol should exist");
        assert_eq!(api.display_name, "Payments API");
        assert_eq!(
            api.description.as_deref(),
            Some("Processes payment requests")
        );
        assert_eq!(api.technology.as_deref(), Some("Axum"));
        assert_eq!(api.tags, vec!["Internal", "HTTP", "Edge"]);
        assert_eq!(api.url.as_deref(), Some("https://example.com/api"));

        let relationship = snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.binding_name.as_deref() == Some("rel"))
            .expect("relationship symbol should exist");
        assert_eq!(relationship.display_name, "Uses");
        assert_eq!(
            relationship.description.as_deref(),
            Some("Body relationship description")
        );
        assert_eq!(relationship.technology.as_deref(), Some("Mutual TLS"));
        assert_eq!(relationship.tags, vec!["Sync", "Critical", "Observed"]);
        assert_eq!(relationship.url.as_deref(), Some("https://example.com/rel"));
    }

    #[test]
    fn analysis_extracts_deployment_hover_metadata_from_positional_attributes() {
        let mut analyzer = DocumentAnalyzer::new();
        let snapshot = analyzer.analyze(DocumentInput::new(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "Payments"
                    }

                    deploymentEnvironment "Live" {
                        edge = deploymentNode "Edge" "Public entrypoint" "Kubernetes" 2 "Public, Regional" {
                            tag "Blue"
                            url "https://example.com/edge"
                            api = softwareSystemInstance system
                        }
                    }
                }
            "#},
        ));

        let edge = snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.binding_name.as_deref() == Some("edge"))
            .expect("deployment node should exist");
        assert_eq!(edge.display_name, "Edge");
        assert_eq!(edge.description.as_deref(), Some("Public entrypoint"));
        assert_eq!(edge.technology.as_deref(), Some("Kubernetes"));
        assert_eq!(edge.tags, vec!["Public", "Regional", "Blue"]);
        assert_eq!(edge.url.as_deref(), Some("https://example.com/edge"));
    }

    #[test]
    fn analysis_extracts_deployment_instance_hover_metadata_from_header_tags() {
        let mut analyzer = DocumentAnalyzer::new();
        let snapshot = analyzer.analyze(DocumentInput::new(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "Payments"

                        deploymentEnvironment "Live" {
                            edge = deploymentNode "Edge" {
                                canary = softwareSystemInstance system blue "Canary" {
                                    tag "Observed"
                                    url "https://example.com/canary"
                                }
                            }
                        }
                    }
                }
            "#},
        ));

        let canary = snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.binding_name.as_deref() == Some("canary"))
            .expect("software system instance should exist");
        assert_eq!(canary.display_name, "canary");
        assert_eq!(canary.description, None);
        assert_eq!(canary.technology, None);
        assert_eq!(canary.tags, vec!["Canary", "Observed"]);
        assert_eq!(canary.url.as_deref(), Some("https://example.com/canary"));
    }

    #[test]
    fn analysis_preserves_empty_relationship_placeholder_slots() {
        let mut analyzer = DocumentAnalyzer::new();
        let snapshot = analyzer.analyze(DocumentInput::new(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                        system = softwareSystem "Payments"

                        rel = user -> system "" "HTTPS" "Async, Observed"
                    }
                }
            "#},
        ));

        let relationship = snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.binding_name.as_deref() == Some("rel"))
            .expect("relationship symbol should exist");
        assert_eq!(relationship.display_name, "rel");
        assert_eq!(relationship.description, None);
        assert_eq!(relationship.technology.as_deref(), Some("HTTPS"));
        assert_eq!(relationship.tags, vec!["Async", "Observed"]);
    }

    #[test]
    fn analysis_preserves_empty_deployment_placeholder_slots() {
        let mut analyzer = DocumentAnalyzer::new();
        let snapshot = analyzer.analyze(DocumentInput::new(
            "workspace.dsl",
            indoc! {r#"
                workspace {
                    model {
                        system = softwareSystem "Payments"
                    }

                    deploymentEnvironment "Live" {
                        edge = deploymentNode "Edge" "" "Kubernetes" 2 "Prod" {
                            api = softwareSystemInstance system
                        }
                    }
                }
            "#},
        ));

        let edge = snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.binding_name.as_deref() == Some("edge"))
            .expect("deployment node should exist");
        assert_eq!(edge.display_name, "Edge");
        assert_eq!(edge.description, None);
        assert_eq!(edge.technology.as_deref(), Some("Kubernetes"));
        assert_eq!(edge.tags, vec!["Prod"]);
    }
}
