// The long-lived analysis session and cache packets that let repeated workspace
// loads reuse document, context, instance, and final-assembly work.

// =============================================================================
// Private analysis session
// =============================================================================
//
// The public loader remains a compatibility facade, but a longer-lived internal
// session now owns document analysis state, open-buffer overrides, and a
// per-path cache of analyzed document packets. This gives future incremental
// work one stable host object rather than rebuilding every internal packet from
// scratch on each load.

#[derive(Default)]
struct WorkspaceAnalysisSession {
    analyzer: DocumentAnalyzer,
    document_overrides: BTreeMap<PathBuf, String>,
    document_cache: BTreeMap<PathBuf, CachedWorkspaceDocument>,
    processed_context_cache: BTreeMap<DocumentContextKey, CachedProcessedDocumentContext>,
    workspace_instance_cache: BTreeMap<DocumentContextKey, CachedWorkspaceInstance>,
    workspace_facts_assembly_cache: Option<CachedWorkspaceFactsAssembly>,
    next_document_generation: u64,
    next_semantic_generation: u64,
    next_context_revision: u64,
}

#[derive(Debug)]
struct CachedWorkspaceDocument {
    snapshot: Arc<crate::DocumentSnapshot>,
    directive_events: Arc<[WorkspaceDirectiveEvent]>,
    semantic_facts: Arc<WorkspaceSemanticDocumentFacts>,
    kind: WorkspaceDocumentKind,
    generation: u64,
    semantic_generation: u64,
}

#[derive(Debug, Clone)]
struct CachedProcessedDocumentContext {
    processed: ProcessedDocumentContext,
    document_generation: u64,
    child_context_revisions: Vec<u64>,
    include_validations: Vec<CachedIncludeValidation>,
    revision: u64,
}

#[derive(Debug, Clone)]
struct CachedWorkspaceInstance {
    document_semantic_generations: Vec<(DocumentId, u64)>,
    derived: Arc<DerivedWorkspaceInstance>,
}

#[derive(Debug, Clone)]
struct CachedWorkspaceFactsAssembly {
    key: WorkspaceFactsAssemblyKey,
    derived: Arc<DerivedWorkspaceFactsAssembly>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RevisionedContextKey {
    context_key: DocumentContextKey,
    revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WorkspaceFactsAssemblyKey {
    processed_contexts: Vec<RevisionedContextKey>,
    workspace_instances: Vec<RevisionedContextKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CachedIncludeValidation {
    RemoteUrl {
        url: String,
    },
    UnsupportedLocalPath {
        path: PathBuf,
    },
    MissingLocalPath {
        path: PathBuf,
    },
    LocalFile {
        path: PathBuf,
        document_generation: u64,
    },
    LocalDirectory {
        path: PathBuf,
        discovered_paths: Vec<(PathBuf, u64)>,
    },
}

impl CachedWorkspaceDocument {
    fn new(
        snapshot: crate::DocumentSnapshot,
        semantic_facts: Arc<WorkspaceSemanticDocumentFacts>,
        generation: u64,
        semantic_generation: u64,
    ) -> Self {
        let directive_events =
            Arc::<[WorkspaceDirectiveEvent]>::from(collect_document_directive_events(&snapshot));
        let kind = if snapshot.is_workspace_entry() {
            WorkspaceDocumentKind::Entry
        } else {
            WorkspaceDocumentKind::Fragment
        };

        Self {
            snapshot: Arc::new(snapshot),
            directive_events,
            semantic_facts,
            kind,
            generation,
            semantic_generation,
        }
    }

    fn workspace_document(&self, discovered_by_scan: bool) -> WorkspaceDocument {
        WorkspaceDocument::new(
            Arc::clone(&self.snapshot),
            Arc::clone(&self.directive_events),
            Arc::clone(&self.semantic_facts),
            self.kind,
            self.semantic_generation,
            discovered_by_scan,
        )
    }
}

impl WorkspaceAnalysisSession {
    fn set_document_override(&mut self, path: PathBuf, source: String) {
        self.document_overrides.insert(path, source);
    }

    fn clear_document_overrides(&mut self) {
        self.document_overrides.clear();
    }

    fn workspace_document(
        &mut self,
        path: &Path,
        discovered_by_scan: bool,
    ) -> io::Result<WorkspaceDocument> {
        let source = self.document_source(path)?;
        let needs_refresh = self
            .document_cache
            .get(path)
            .is_none_or(|cached| cached.snapshot.source() != source);

        if needs_refresh {
            let snapshot = self.analyzer.analyze(
                DocumentInput::new(document_id_from_path(path), source)
                    .with_location(path.to_path_buf()),
            );
            let semantic_facts = Arc::new(WorkspaceSemanticDocumentFacts::from_snapshot(&snapshot));
            let generation = self.next_document_generation();
            let previous_semantic_generation = self
                .document_cache
                .get(path)
                .filter(|cached| cached.semantic_facts.as_ref() == semantic_facts.as_ref())
                .map(|cached| cached.semantic_generation);
            let semantic_generation =
                previous_semantic_generation.unwrap_or_else(|| self.next_semantic_generation());
            self.document_cache.insert(
                path.to_path_buf(),
                CachedWorkspaceDocument::new(
                    snapshot,
                    semantic_facts,
                    generation,
                    semantic_generation,
                ),
            );
        }

        Ok(self
            .document_cache
            .get(path)
            .expect("BUG: workspace document cache entry should exist after refresh")
            .workspace_document(discovered_by_scan))
    }

    fn document_source(&self, path: &Path) -> io::Result<String> {
        self.document_overrides
            .get(path)
            .map_or_else(|| fs::read_to_string(path), |source| Ok(source.clone()))
    }

    fn document_generation(&self, path: &Path) -> Option<u64> {
        self.document_cache
            .get(path)
            .map(|cached| cached.generation)
    }

    fn processed_context_revision(&self, key: &DocumentContextKey) -> Option<u64> {
        self.processed_context_cache
            .get(key)
            .map(|cached| cached.revision)
    }

    fn cached_processed_context(
        &self,
        key: &DocumentContextKey,
    ) -> Option<CachedProcessedDocumentContext> {
        self.processed_context_cache.get(key).cloned()
    }

    fn cached_workspace_instance(
        &self,
        key: &DocumentContextKey,
    ) -> Option<CachedWorkspaceInstance> {
        self.workspace_instance_cache.get(key).cloned()
    }

    fn cached_workspace_facts_assembly(
        &self,
        key: &WorkspaceFactsAssemblyKey,
    ) -> Option<Arc<DerivedWorkspaceFactsAssembly>> {
        self.workspace_facts_assembly_cache
            .as_ref()
            .filter(|cached| cached.key == *key)
            .map(|cached| Arc::clone(&cached.derived))
    }

    fn store_processed_context(
        &mut self,
        context: &DocumentContext,
        processed: ProcessedDocumentContext,
    ) {
        let child_context_revisions = processed_context_dependency_keys(&processed)
            .map(|child_context| {
                self.processed_context_revision(child_context)
                    .expect("BUG: child context should be cached before its parent")
            })
            .collect();
        let include_validations = processed
            .direct_includes
            .iter()
            .map(|include| self.include_validation(include))
            .collect();
        let cached = CachedProcessedDocumentContext {
            document_generation: self
                .document_generation(&context.path)
                .expect("BUG: processed context document should already be loaded"),
            processed,
            child_context_revisions,
            include_validations,
            revision: self.next_context_revision(),
        };

        self.processed_context_cache
            .insert(context.key.clone(), cached);
    }

    fn store_workspace_instance(
        &mut self,
        root_context_key: &DocumentContextKey,
        document_semantic_generations: Vec<(DocumentId, u64)>,
        derived: Arc<DerivedWorkspaceInstance>,
    ) {
        let cached = CachedWorkspaceInstance {
            document_semantic_generations,
            derived,
        };

        self.workspace_instance_cache
            .insert(root_context_key.clone(), cached);
    }

    fn store_workspace_facts_assembly(
        &mut self,
        key: WorkspaceFactsAssemblyKey,
        derived: Arc<DerivedWorkspaceFactsAssembly>,
    ) {
        self.workspace_facts_assembly_cache = Some(CachedWorkspaceFactsAssembly { key, derived });
    }

    fn include_validation(&self, include: &ResolvedInclude) -> CachedIncludeValidation {
        match include.target() {
            WorkspaceIncludeTarget::RemoteUrl { url } => {
                CachedIncludeValidation::RemoteUrl { url: url.clone() }
            }
            WorkspaceIncludeTarget::UnsupportedLocalPath { path } => {
                CachedIncludeValidation::UnsupportedLocalPath { path: path.clone() }
            }
            WorkspaceIncludeTarget::MissingLocalPath { path } => {
                CachedIncludeValidation::MissingLocalPath { path: path.clone() }
            }
            WorkspaceIncludeTarget::LocalFile { path } => CachedIncludeValidation::LocalFile {
                path: path.clone(),
                document_generation: self
                    .document_generation(path)
                    .expect("BUG: local include file should already be loaded"),
            },
            WorkspaceIncludeTarget::LocalDirectory { path } => {
                CachedIncludeValidation::LocalDirectory {
                    path: path.clone(),
                    discovered_paths: include
                        .discovered_documents()
                        .iter()
                        .map(|document_id| {
                            let path = PathBuf::from(document_id.as_str());
                            let generation = self
                                .document_generation(&path)
                                .expect("BUG: directory include child should already be loaded");
                            (path, generation)
                        })
                        .collect(),
                }
            }
        }
    }

    const fn next_document_generation(&mut self) -> u64 {
        self.next_document_generation = self
            .next_document_generation
            .checked_add(1)
            .expect("document generation counter should not overflow");
        self.next_document_generation
    }

    const fn next_semantic_generation(&mut self) -> u64 {
        self.next_semantic_generation = self
            .next_semantic_generation
            .checked_add(1)
            .expect("semantic generation counter should not overflow");
        self.next_semantic_generation
    }

    const fn next_context_revision(&mut self) -> u64 {
        self.next_context_revision = self
            .next_context_revision
            .checked_add(1)
            .expect("context revision counter should not overflow");
        self.next_context_revision
    }
}

impl CachedWorkspaceInstance {
    fn workspace_index(&self, id: WorkspaceInstanceId) -> WorkspaceIndex {
        WorkspaceIndex::from_derived(id, Arc::clone(&self.derived))
    }
}

#[cfg(test)]
mod workspace_session_tests {
    use super::*;

    use indoc::indoc;
    use std::sync::Arc;

    #[test]
    fn loader_reuses_cached_workspace_instance_payloads_across_identical_loads() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                !include "model.dsl"
            }
        "#});
        fixture.write_model(indoc! {r#"
            model {
                user = person "User"
            }
        "#});

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.root()])
            .expect("first load should succeed");
        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");

        let first_index = first
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        let second_index = second
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");

        assert!(Arc::ptr_eq(&first_index.derived, &second_index.derived));
    }

    #[test]
    fn source_only_edit_reuses_workspace_semantics_when_syntax_facts_are_unchanged() {
        let source = indoc! {r#"
            workspace {
                model {
                    user = person "User"
                }
            }
        "#};
        let fixture = TemporaryWorkspace::new(source);

        let mut loader = WorkspaceLoader::new();

        let first = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("first load should succeed");

        loader.set_document_override(fixture.workspace_path().clone(), format!("{source}\n"));

        let second = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("override-backed load should succeed");

        let first_index = first
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        let second_index = second
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");

        assert!(Arc::ptr_eq(&first_index.derived, &second_index.derived));
    }

    #[test]
    fn loader_reuses_final_workspace_facts_assembly_across_identical_loads() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                !include "model.dsl"
            }
        "#});
        fixture.write_model(indoc! {r#"
            model {
                user = person "User"
            }
        "#});

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.root()])
            .expect("first load should succeed");
        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");

        assert!(Arc::ptr_eq(first.assembly_arc(), second.assembly_arc()));
    }

    #[test]
    fn sibling_root_changes_only_refresh_the_affected_workspace_instance() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                model {
                    user = person "User"
                }
            }
        "#});
        fixture.write_file(
            "other.dsl",
            indoc! {r#"
                workspace {
                    model {
                        admin = person "Admin"
                    }
                }
            "#},
        );
        let other_path = fixture
            .root()
            .join("other.dsl")
            .canonicalize()
            .expect("other workspace path should canonicalize");

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.root()])
            .expect("first load should succeed");

        fixture.write_file(
            "other.dsl",
            indoc! {r#"
                workspace {
                    model {
                        support = person "Support"
                    }
                }
            "#},
        );

        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");

        let first_workspace = workspace_index_for_root(&first, fixture.workspace_path());
        let second_workspace = workspace_index_for_root(&second, fixture.workspace_path());
        let first_other = workspace_index_for_root(&first, &other_path);
        let second_other = workspace_index_for_root(&second, &other_path);

        assert!(Arc::ptr_eq(
            &first_workspace.derived,
            &second_workspace.derived
        ));
        assert!(!Arc::ptr_eq(&first_other.derived, &second_other.derived));
        assert_eq!(
            second_other
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["support"]
        );
    }

    #[test]
    fn sibling_root_changes_refresh_final_assembly_when_membership_changes() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                model {
                    user = person "User"
                }
            }
        "#});
        fixture.write_file(
            "other.dsl",
            indoc! {r#"
                workspace {
                    !include "shared.dsl"
                }
            "#},
        );
        fixture.write_file(
            "shared.dsl",
            indoc! {r#"
                model {
                    admin = person "Admin"
                }
            "#},
        );

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.root()])
            .expect("first load should succeed");

        fixture.write_file(
            "other.dsl",
            indoc! {r#"
                workspace {
                    model {
                        support = person "Support"
                    }
                }
            "#},
        );

        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");

        assert!(!Arc::ptr_eq(first.assembly_arc(), second.assembly_arc()));
        assert_eq!(
            second
                .candidate_instances_for(&document_id_from_path(&fixture.root().join("shared.dsl")))
                .count(),
            0
        );
    }
}
