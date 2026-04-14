//! Workspace discovery, include-following, and file-level include diagnostics.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use ignore::WalkBuilder;

use crate::{
    Annotation, ConstantDefinition, DocumentAnalyzer, DocumentId, DocumentInput, IdentifierMode,
    IdentifierModeFact, IncludeDirective, Reference, ReferenceKind, ReferenceTargetHint,
    RuledDiagnostic, Symbol, SymbolId, SymbolKind, TextSpan,
    includes::{DirectiveContainer, DirectiveValueKind, normalized_directive_value},
    semantic::{
        ConfigurationScopeFact, DynamicViewStepFact, ElementDirectiveFact, ImageSourceKind,
        PropertyFact, RelationshipFact, ResourceDirectiveFact, ResourceDirectiveKind, ValueFact,
        ViewFact, ViewKind, WorkspaceScope, WorkspaceSectionFact, WorkspaceSectionKind,
    },
};

/// Classifies whether a discovered document can act as a workspace entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceDocumentKind {
    /// A document with a top-level `workspace { ... }` block.
    Entry,
    /// A parseable fragment that does not declare a top-level workspace.
    Fragment,
}

/// One discovered document plus the metadata gathered during workspace loading.
#[derive(Debug)]
pub struct WorkspaceDocument {
    snapshot: Arc<crate::DocumentSnapshot>,
    semantic_facts: Arc<WorkspaceSemanticDocumentFacts>,
    kind: WorkspaceDocumentKind,
    semantic_generation: u64,
    discovered_by_scan: bool,
}

impl WorkspaceDocument {
    const fn new(
        snapshot: Arc<crate::DocumentSnapshot>,
        semantic_facts: Arc<WorkspaceSemanticDocumentFacts>,
        kind: WorkspaceDocumentKind,
        semantic_generation: u64,
        discovered_by_scan: bool,
    ) -> Self {
        Self {
            snapshot,
            semantic_facts,
            kind,
            semantic_generation,
            discovered_by_scan,
        }
    }

    /// Returns the stable document identifier for the discovered document.
    #[must_use]
    pub fn id(&self) -> &DocumentId {
        self.snapshot.id()
    }

    /// Returns the analyzed snapshot for the discovered document.
    #[must_use]
    pub fn snapshot(&self) -> &crate::DocumentSnapshot {
        self.snapshot.as_ref()
    }

    fn semantic_facts(&self) -> &WorkspaceSemanticDocumentFacts {
        self.semantic_facts.as_ref()
    }

    const fn semantic_generation(&self) -> u64 {
        self.semantic_generation
    }

    /// Returns the document's role in the discovered workspace set.
    #[must_use]
    pub const fn kind(&self) -> WorkspaceDocumentKind {
        self.kind
    }

    /// Returns whether broad `.dsl` workspace scanning found this document.
    ///
    /// Documents discovered only via explicit `!include` traversal report
    /// `false` here.
    #[must_use]
    pub const fn discovered_by_scan(&self) -> bool {
        self.discovered_by_scan
    }

    const fn mark_discovered_by_scan(&mut self) {
        self.discovered_by_scan = true;
    }
}

/// The normalized target shape observed for one explicit `!include`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceIncludeTarget {
    /// A local file include that resolved to one concrete path.
    LocalFile {
        /// Canonical filesystem path to the followed local file.
        path: PathBuf,
    },
    /// A local directory include that expanded to zero or more concrete files.
    LocalDirectory {
        /// Canonical filesystem path to the followed local directory.
        path: PathBuf,
    },
    /// A remote include target that is recorded but not fetched.
    RemoteUrl {
        /// Remote HTTPS URL recorded during discovery.
        url: String,
    },
    /// A relative local target that did not exist on disk.
    MissingLocalPath {
        /// Filesystem path that did not exist when discovery attempted to follow it.
        path: PathBuf,
    },
    /// A local target that was rejected before loading, such as an absolute path
    /// or one that escapes the including document's directory tree.
    UnsupportedLocalPath {
        /// Filesystem path that discovery refused to follow.
        path: PathBuf,
    },
}

/// One include directive plus the discovery-layer result of following it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInclude {
    including_document: DocumentId,
    raw_value: String,
    target_text: String,
    span: TextSpan,
    value_span: TextSpan,
    target: WorkspaceIncludeTarget,
    discovered_documents: Vec<DocumentId>,
}

impl ResolvedInclude {
    /// Returns the document that declared this include directive.
    #[must_use]
    pub const fn including_document(&self) -> &DocumentId {
        &self.including_document
    }

    /// Returns the exact directive value text as it appeared in the document.
    #[must_use]
    pub fn raw_value(&self) -> &str {
        &self.raw_value
    }

    /// Returns the normalized target text after string substitution.
    #[must_use]
    pub fn target_text(&self) -> &str {
        &self.target_text
    }

    /// Returns the span of the full include directive.
    #[must_use]
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    /// Returns the span of the include directive's value node.
    #[must_use]
    pub const fn value_span(&self) -> TextSpan {
        self.value_span
    }

    /// Returns the normalized target classification observed during discovery.
    #[must_use]
    pub const fn target(&self) -> &WorkspaceIncludeTarget {
        &self.target
    }

    /// Returns the documents followed from this include target, if any.
    #[must_use]
    pub fn discovered_documents(&self) -> &[DocumentId] {
        &self.discovered_documents
    }
}

/// Stable identity for one derived workspace instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceInstanceId(usize);

impl WorkspaceInstanceId {
    /// Returns the stable numeric identity assigned during one workspace load.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Stable reference to one extracted symbol in one discovered document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolHandle {
    document: DocumentId,
    symbol_id: SymbolId,
}

impl SymbolHandle {
    /// Creates a stable handle for one symbol in one discovered document.
    #[must_use]
    pub fn new(document: impl Into<DocumentId>, symbol_id: SymbolId) -> Self {
        Self {
            document: document.into(),
            symbol_id,
        }
    }

    /// Returns the document that owns the referenced symbol.
    #[must_use]
    pub const fn document(&self) -> &DocumentId {
        &self.document
    }

    /// Returns the snapshot-local symbol identifier.
    #[must_use]
    pub const fn symbol_id(&self) -> SymbolId {
        self.symbol_id
    }
}

/// Stable reference to one extracted reference site in one discovered document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReferenceHandle {
    document: DocumentId,
    reference_index: usize,
}

impl ReferenceHandle {
    /// Creates a stable handle for one reference site in one discovered document.
    #[must_use]
    pub fn new(document: impl Into<DocumentId>, reference_index: usize) -> Self {
        Self {
            document: document.into(),
            reference_index,
        }
    }

    /// Returns the document that owns the referenced syntax site.
    #[must_use]
    pub const fn document(&self) -> &DocumentId {
        &self.document
    }

    /// Returns the snapshot-local reference index.
    #[must_use]
    pub const fn reference_index(&self) -> usize {
        self.reference_index
    }
}

/// Explains how one supported reference site resolved inside a workspace index.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceResolutionStatus {
    /// The reference resolved confidently to one declaration symbol.
    Resolved(SymbolHandle),
    /// No matching binding existed in the relevant semantic table.
    UnresolvedNoMatch,
    /// One or more duplicate bindings prevented a confident answer.
    AmbiguousDuplicateBinding,
    /// Element and relationship bindings both matched the same raw text.
    AmbiguousElementVsRelationship,
    /// The current scope rules intentionally defer this reference surface.
    DeferredByScopePolicy,
}

// The semantic payload for one workspace instance is reusable across loads, but
// `WorkspaceInstanceId` is intentionally load-local. Keeping the payload in its
// own packet lets repeated loads share expensive binding/reference work without
// smuggling a stale per-load id through the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DerivedWorkspaceInstance {
    root_document: DocumentId,
    documents: Vec<DocumentId>,
    element_identifier_modes: BTreeMap<DocumentId, ElementIdentifierMode>,
    unique_element_bindings: BTreeMap<String, SymbolHandle>,
    duplicate_element_bindings: BTreeMap<String, Vec<SymbolHandle>>,
    unique_deployment_bindings: BTreeMap<String, SymbolHandle>,
    duplicate_deployment_bindings: BTreeMap<String, Vec<SymbolHandle>>,
    unique_relationship_bindings: BTreeMap<String, SymbolHandle>,
    duplicate_relationship_bindings: BTreeMap<String, Vec<SymbolHandle>>,
    reference_resolutions: BTreeMap<ReferenceHandle, ReferenceResolutionStatus>,
    references_by_target: BTreeMap<SymbolHandle, Vec<ReferenceHandle>>,
    semantic_diagnostics: Vec<RuledDiagnostic>,
}

// Keep the expensive per-instance binding/reference pass anchored to a stable
// semantic packet derived from `DocumentSyntaxFacts`. When a document source edit
// produces the same semantic packet, the host-side workspace caches can now
// reuse the derived instance payload without paying the full rebuild cost again.
//
// This stays separate from the public `DocumentSyntaxFacts` surface on purpose.
// `DocumentSyntaxFacts` is the reusable document-analysis boundary, while this
// packet is the workspace session's private cache payload for comparing and
// cloning the semantic subset that drives binding/reference assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceSemanticDocumentFacts {
    document_id: DocumentId,
    has_syntax_errors: bool,
    identifier_modes: Vec<IdentifierModeFact>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
    workspace_sections: Vec<WorkspaceSectionFact>,
    configuration_scopes: Vec<ConfigurationScopeFact>,
    property_facts: Vec<PropertyFact>,
    resource_directives: Vec<ResourceDirectiveFact>,
    element_directives: Vec<ElementDirectiveFact>,
    relationship_facts: Vec<RelationshipFact>,
    view_facts: Vec<ViewFact>,
}

impl WorkspaceSemanticDocumentFacts {
    fn from_snapshot(snapshot: &crate::DocumentSnapshot) -> Self {
        Self {
            document_id: snapshot.id().clone(),
            has_syntax_errors: snapshot.has_syntax_errors(),
            identifier_modes: snapshot.identifier_modes().to_vec(),
            symbols: snapshot.symbols().to_vec(),
            references: snapshot.references().to_vec(),
            workspace_sections: snapshot.workspace_sections().to_vec(),
            configuration_scopes: snapshot.configuration_scopes().to_vec(),
            property_facts: snapshot.property_facts().to_vec(),
            resource_directives: snapshot.resource_directives().to_vec(),
            element_directives: snapshot.element_directives().to_vec(),
            relationship_facts: snapshot.relationship_facts().to_vec(),
            view_facts: snapshot.view_facts().to_vec(),
        }
    }
}

/// Derived semantic index for one workspace instance.
#[derive(Debug)]
pub struct WorkspaceIndex {
    id: WorkspaceInstanceId,
    derived: Arc<DerivedWorkspaceInstance>,
}

impl WorkspaceIndex {
    const fn from_derived(id: WorkspaceInstanceId, derived: Arc<DerivedWorkspaceInstance>) -> Self {
        Self { id, derived }
    }

    /// Returns this index's stable instance identity.
    #[must_use]
    pub const fn id(&self) -> WorkspaceInstanceId {
        self.id
    }

    /// Returns the root document that defines this workspace instance.
    #[must_use]
    pub fn root_document(&self) -> &DocumentId {
        &self.derived.root_document
    }

    /// Returns the discovered documents that participate in this instance.
    #[must_use]
    pub fn documents(&self) -> &[DocumentId] {
        &self.derived.documents
    }

    /// Returns the unique element-binding table keyed by canonical binding key.
    #[must_use]
    pub fn unique_element_bindings(&self) -> &BTreeMap<String, SymbolHandle> {
        &self.derived.unique_element_bindings
    }

    /// Returns the effective element-identifier mode for one document in this workspace instance.
    #[must_use]
    pub fn element_identifier_mode_for(
        &self,
        document: &DocumentId,
    ) -> Option<ElementIdentifierMode> {
        self.derived.element_identifier_modes.get(document).copied()
    }

    /// Returns the duplicate element-binding sets keyed by canonical binding key.
    #[must_use]
    pub fn duplicate_element_bindings(&self) -> &BTreeMap<String, Vec<SymbolHandle>> {
        &self.derived.duplicate_element_bindings
    }

    /// Returns the unique deployment-binding table keyed by binding identifier.
    #[must_use]
    pub fn unique_deployment_bindings(&self) -> &BTreeMap<String, SymbolHandle> {
        &self.derived.unique_deployment_bindings
    }

    /// Returns the duplicate deployment-binding sets keyed by binding identifier.
    #[must_use]
    pub fn duplicate_deployment_bindings(&self) -> &BTreeMap<String, Vec<SymbolHandle>> {
        &self.derived.duplicate_deployment_bindings
    }

    /// Returns the unique relationship-binding table keyed by canonical binding key.
    #[must_use]
    pub fn unique_relationship_bindings(&self) -> &BTreeMap<String, SymbolHandle> {
        &self.derived.unique_relationship_bindings
    }

    /// Returns the duplicate relationship-binding sets keyed by canonical key.
    #[must_use]
    pub fn duplicate_relationship_bindings(&self) -> &BTreeMap<String, Vec<SymbolHandle>> {
        &self.derived.duplicate_relationship_bindings
    }

    /// Returns the resolution status recorded for one reference handle.
    #[must_use]
    pub fn reference_resolution(
        &self,
        handle: &ReferenceHandle,
    ) -> Option<&ReferenceResolutionStatus> {
        self.derived.reference_resolutions.get(handle)
    }

    /// Returns every resolved reference that points at one symbol.
    pub fn references_for_symbol(
        &self,
        handle: &SymbolHandle,
    ) -> impl Iterator<Item = &ReferenceHandle> + '_ {
        self.derived
            .references_by_target
            .get(handle)
            .into_iter()
            .flatten()
    }

    /// Returns the semantic diagnostics derived for this workspace instance.
    #[must_use]
    pub fn semantic_diagnostics(&self) -> &[RuledDiagnostic] {
        &self.derived.semantic_diagnostics
    }

    /// Returns whether the workspace instance includes one document.
    #[must_use]
    pub fn contains_document(&self, document: &DocumentId) -> bool {
        self.derived.documents.contains(document)
    }
}

#[derive(Debug, Clone, Default)]
struct DerivedWorkspaceFactsAssembly {
    resolved_includes: Vec<ResolvedInclude>,
    include_diagnostics: Vec<RuledDiagnostic>,
    document_instances: BTreeMap<DocumentId, Vec<WorkspaceInstanceId>>,
    semantic_diagnostics: Vec<RuledDiagnostic>,
}

/// Multi-file discovery facts gathered from one or more workspace roots.
#[derive(Debug, Default)]
pub struct WorkspaceFacts {
    documents: Vec<WorkspaceDocument>,
    workspace_indexes: Vec<WorkspaceIndex>,
    assembly: Arc<DerivedWorkspaceFactsAssembly>,
}

impl WorkspaceFacts {
    #[cfg(test)]
    const fn assembly_arc(&self) -> &Arc<DerivedWorkspaceFactsAssembly> {
        &self.assembly
    }

    /// Returns every discovered document in deterministic path order.
    #[must_use]
    pub fn documents(&self) -> &[WorkspaceDocument] {
        &self.documents
    }

    /// Returns the discovered include-following results in deterministic order.
    #[must_use]
    pub fn includes(&self) -> &[ResolvedInclude] {
        &self.assembly.resolved_includes
    }

    /// Returns include-resolution diagnostics in deterministic order.
    #[must_use]
    pub fn include_diagnostics(&self) -> &[RuledDiagnostic] {
        &self.assembly.include_diagnostics
    }

    /// Returns include-resolution diagnostics for one document.
    pub fn include_diagnostics_for(
        &self,
        id: &DocumentId,
    ) -> impl Iterator<Item = &RuledDiagnostic> + '_ {
        let id = id.clone();
        self.assembly
            .include_diagnostics
            .iter()
            .filter(move |diagnostic| diagnostic.document() == Some(&id))
    }

    /// Returns the subset of discovered documents that can act as entry roots.
    pub fn entry_documents(&self) -> impl Iterator<Item = &WorkspaceDocument> + '_ {
        self.documents
            .iter()
            .filter(|document| document.kind() == WorkspaceDocumentKind::Entry)
    }

    /// Looks up one discovered document by document identifier.
    #[must_use]
    pub fn document(&self, id: &DocumentId) -> Option<&WorkspaceDocument> {
        self.documents.iter().find(|document| document.id() == id)
    }

    /// Returns the derived workspace indexes keyed by entry/root document.
    #[must_use]
    pub fn workspace_indexes(&self) -> &[WorkspaceIndex] {
        &self.workspace_indexes
    }

    /// Looks up one derived workspace index by instance identity.
    #[must_use]
    pub fn workspace_index(&self, id: WorkspaceInstanceId) -> Option<&WorkspaceIndex> {
        self.workspace_indexes.iter().find(|index| index.id() == id)
    }

    /// Returns the candidate workspace instances that include one document.
    pub fn candidate_instances_for(
        &self,
        id: &DocumentId,
    ) -> impl Iterator<Item = &WorkspaceInstanceId> + '_ {
        self.assembly
            .document_instances
            .get(id)
            .into_iter()
            .flatten()
    }

    /// Returns every merged semantic diagnostic in deterministic order.
    #[must_use]
    pub fn semantic_diagnostics(&self) -> &[RuledDiagnostic] {
        &self.assembly.semantic_diagnostics
    }

    /// Returns merged semantic diagnostics for one document.
    pub fn semantic_diagnostics_for(
        &self,
        id: &DocumentId,
    ) -> impl Iterator<Item = &RuledDiagnostic> + '_ {
        let id = id.clone();
        self.assembly
            .semantic_diagnostics
            .iter()
            .filter(move |diagnostic| diagnostic.document() == Some(&id))
    }
}

/// Loader that scans workspace roots and follows explicit include targets.
#[derive(Default)]
pub struct WorkspaceLoader {
    session: WorkspaceAnalysisSession,
}

impl WorkspaceLoader {
    /// Creates a loader with a reusable parser-backed document analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an in-memory source override for one canonical file path.
    ///
    /// This is useful for editor integrations that need workspace loading to
    /// observe unsaved buffer contents instead of only on-disk text.
    pub fn set_document_override(&mut self, path: PathBuf, source: String) {
        self.session.set_document_override(path, source);
    }

    /// Clears all registered in-memory source overrides.
    ///
    /// This keeps a long-lived loader aligned with the current open-buffer set.
    pub fn clear_document_overrides(&mut self) {
        self.session.clear_document_overrides();
    }

    /// Scans one or more workspace roots for `.dsl` files and follows explicit
    /// local include targets from discovered documents.
    ///
    /// General workspace scanning respects normal ignore rules. Explicit local
    /// include targets are followed separately and therefore bypass those broad
    /// scan filters.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the loader cannot traverse a workspace root or
    /// read one of the discovered local files.
    pub fn load_paths<I, P>(&mut self, roots: I) -> io::Result<WorkspaceFacts>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut normalized_roots = roots
            .into_iter()
            .map(|root| normalize_existing_path(root.as_ref()))
            .collect::<io::Result<Vec<_>>>()?;
        normalized_roots.sort();
        normalized_roots.dedup();

        WorkspaceBuildSession::new(&mut self.session).build_from_roots(&normalized_roots)
    }
}

impl std::fmt::Debug for WorkspaceLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceLoader").finish_non_exhaustive()
    }
}

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
        let kind = if snapshot.is_workspace_entry() {
            WorkspaceDocumentKind::Entry
        } else {
            WorkspaceDocumentKind::Fragment
        };

        Self {
            snapshot: Arc::new(snapshot),
            semantic_facts,
            kind,
            generation,
            semantic_generation,
        }
    }

    fn workspace_document(&self, discovered_by_scan: bool) -> WorkspaceDocument {
        WorkspaceDocument::new(
            Arc::clone(&self.snapshot),
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

// =============================================================================
// Private workspace-build session
// =============================================================================
//
// `WorkspaceLoader` remains the public compatibility facade, but the mutable
// state for one workspace load now lives in a dedicated session object. This is
// the next Salsa-oriented seam: future incremental workspace inputs and cached
// instance results can hang off this session without changing existing callers.

struct WorkspaceBuildSession<'loader> {
    session: &'loader mut WorkspaceAnalysisSession,
    loaded_documents: BTreeMap<PathBuf, WorkspaceDocument>,
    processed_contexts: BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    active_stack: Vec<PathBuf>,
}

impl<'loader> WorkspaceBuildSession<'loader> {
    const fn new(session: &'loader mut WorkspaceAnalysisSession) -> Self {
        Self {
            session,
            loaded_documents: BTreeMap::new(),
            processed_contexts: BTreeMap::new(),
            active_stack: Vec::new(),
        }
    }

    fn build_from_roots(mut self, normalized_roots: &[PathBuf]) -> io::Result<WorkspaceFacts> {
        // Phase 1: Normalize and scan the requested roots so broad workspace
        // discovery respects ignore rules before include traversal begins.
        self.scan_roots(normalized_roots)?;

        // Phase 2: Re-process the discovered documents in directive order so
        // constants, includes, and cycle detection follow the DSL's imperative
        // execution model.
        let start_contexts = self.process_start_contexts(normalized_roots)?;

        // Phase 3: Flatten the per-document include results into one stable
        // view for downstream diagnostics and editor features.
        Ok(self.finish(&start_contexts))
    }

    fn scan_roots(&mut self, normalized_roots: &[PathBuf]) -> io::Result<()> {
        for root in normalized_roots {
            for path in scan_workspace_root(root)? {
                self.load_document(path, true)?;
            }
        }

        Ok(())
    }

    fn process_start_contexts(
        &mut self,
        normalized_roots: &[PathBuf],
    ) -> io::Result<Vec<DocumentContext>> {
        let start_contexts = start_contexts(normalized_roots, &self.loaded_documents);

        for context in &start_contexts {
            let _ = self.process_document_context(context.clone())?;
        }

        Ok(start_contexts)
    }

    fn finish(self, start_contexts: &[DocumentContext]) -> WorkspaceFacts {
        let workspace_indexes = build_workspace_indexes(
            self.session,
            &self.loaded_documents,
            start_contexts,
            &self.processed_contexts,
        );
        let assembly_key =
            workspace_facts_assembly_key(self.session, start_contexts, &self.processed_contexts);
        let assembly = build_workspace_facts_assembly(
            self.session,
            assembly_key,
            &self.processed_contexts,
            &workspace_indexes,
        );

        WorkspaceFacts {
            documents: self.loaded_documents.into_values().collect(),
            workspace_indexes,
            assembly,
        }
    }

    fn load_document(&mut self, path: PathBuf, discovered_by_scan: bool) -> io::Result<()> {
        if let Some(document) = self.loaded_documents.get_mut(&path) {
            if discovered_by_scan {
                document.mark_discovered_by_scan();
            }
            return Ok(());
        }

        let document = self.session.workspace_document(&path, discovered_by_scan)?;
        self.loaded_documents.insert(path, document);
        Ok(())
    }

    fn process_document_context(
        &mut self,
        context: DocumentContext,
    ) -> io::Result<ConstantEnvironment> {
        // Memoize by `(path, inherited constants)` so repeated includes can
        // share the same processed result without rewalking the document.
        if let Some(processed_context) = self.processed_contexts.get(&context.key) {
            return Ok(processed_context.exported_constants.clone());
        }

        self.load_document(context.path.clone(), false)?;

        if self.cached_context_tree_is_fresh(&context.key, &mut BTreeSet::new())? {
            self.materialize_cached_context_tree(&context.key, &mut BTreeSet::new())?;
            return Ok(self
                .processed_contexts
                .get(&context.key)
                .expect("BUG: fresh cached context should materialize into the current load")
                .exported_constants
                .clone());
        }

        let (document_id, workspace_base, constant_definitions, include_directives) = {
            let document = self
                .loaded_documents
                .get(&context.path)
                .expect("BUG: document context should be loaded before processing");

            (
                document.id().clone(),
                workspace_base_directive(document.snapshot()),
                document.snapshot().constant_definitions().to_vec(),
                document.snapshot().include_directives().to_vec(),
            )
        };

        self.active_stack.push(context.path.clone());
        let processed = (|| -> io::Result<ProcessedDocumentContext> {
            let mut current_constants = context.inherited_constants.clone();
            let mut workspace_base_context = None;
            let mut direct_includes = Vec::new();
            let mut included_contexts = Vec::new();

            // Load an extended base workspace before the current document's own
            // directives so inherited constants and bindings mirror the DSL's
            // top-down `workspace extends ...` semantics.
            if let Some(workspace_base) = &workspace_base {
                let base_path =
                    resolve_workspace_base(&context.path, workspace_base, &current_constants)?;
                self.load_document(base_path.clone(), false)?;

                if self.active_stack.contains(&base_path) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "workspace extends cycle detected while following: {}",
                            base_path.display()
                        ),
                    ));
                }

                let child_context = DocumentContext::new(base_path, current_constants.clone());
                workspace_base_context = Some(child_context.key.clone());
                current_constants = self.process_document_context(child_context)?;
            }

            // Process constants and includes in source order so inherited values,
            // local definitions, and included fragments all obey the DSL's
            // imperative execution model.
            for event in document_directive_events(&constant_definitions, &include_directives) {
                match event {
                    DocumentDirectiveEvent::Constant(constant) => {
                        apply_constant_definition(constant, &mut current_constants);
                    }
                    DocumentDirectiveEvent::Include(directive) => {
                        let resolved_include = resolve_include(
                            &document_id,
                            &context.path,
                            directive,
                            &current_constants,
                        )?;

                        for included_path in &resolved_include.discovered_paths {
                            self.load_document(included_path.clone(), false)?;
                        }

                        for included_path in &resolved_include.discovered_paths {
                            if self.active_stack.contains(included_path) {
                                continue;
                            }

                            let child_context = DocumentContext::new(
                                included_path.clone(),
                                current_constants.clone(),
                            );
                            included_contexts.push(child_context.key.clone());
                            current_constants = self.process_document_context(child_context)?;
                        }

                        direct_includes.push(resolved_include.include);
                    }
                }
            }

            Ok(ProcessedDocumentContext {
                exported_constants: current_constants,
                workspace_base_context,
                direct_includes,
                included_contexts,
            })
        })();
        let popped_path = self.active_stack.pop();
        debug_assert_eq!(popped_path.as_deref(), Some(context.path.as_path()));

        let processed = processed?;
        let exported_constants = processed.exported_constants.clone();
        self.session
            .store_processed_context(&context, processed.clone());
        self.processed_contexts.insert(context.key, processed);
        Ok(exported_constants)
    }

    fn cached_context_tree_is_fresh(
        &mut self,
        context_key: &DocumentContextKey,
        visiting: &mut BTreeSet<DocumentContextKey>,
    ) -> io::Result<bool> {
        if !visiting.insert(context_key.clone()) {
            return Ok(true);
        }

        let freshness = (|| -> io::Result<bool> {
            let Some(cached) = self.session.cached_processed_context(context_key) else {
                return Ok(false);
            };

            self.load_document(context_key.path.clone(), false)?;

            let Some(current_generation) = self.session.document_generation(&context_key.path)
            else {
                return Ok(false);
            };
            if current_generation != cached.document_generation {
                return Ok(false);
            }

            for (child_context, expected_revision) in
                processed_context_dependency_keys(&cached.processed)
                    .zip(&cached.child_context_revisions)
            {
                let Some(current_revision) = self.session.processed_context_revision(child_context)
                else {
                    return Ok(false);
                };
                if current_revision != *expected_revision {
                    return Ok(false);
                }
                if !self.cached_context_tree_is_fresh(child_context, visiting)? {
                    return Ok(false);
                }
            }

            for validation in &cached.include_validations {
                if !self.include_validation_is_fresh(validation)? {
                    return Ok(false);
                }
            }

            Ok(true)
        })();
        let _ = visiting.remove(context_key);
        freshness
    }

    fn materialize_cached_context_tree(
        &mut self,
        context_key: &DocumentContextKey,
        visiting: &mut BTreeSet<DocumentContextKey>,
    ) -> io::Result<()> {
        if self.processed_contexts.contains_key(context_key) {
            return Ok(());
        }
        if !visiting.insert(context_key.clone()) {
            return Ok(());
        }

        let materialized = (|| -> io::Result<()> {
            self.load_document(context_key.path.clone(), false)?;

            let cached = self
                .session
                .cached_processed_context(context_key)
                .expect("BUG: fresh cached context should still exist while materializing");

            for child_context in processed_context_dependency_keys(&cached.processed) {
                self.materialize_cached_context_tree(child_context, visiting)?;
            }

            self.processed_contexts
                .insert(context_key.clone(), cached.processed);
            Ok(())
        })();
        let _ = visiting.remove(context_key);
        materialized
    }

    fn include_validation_is_fresh(
        &mut self,
        validation: &CachedIncludeValidation,
    ) -> io::Result<bool> {
        match validation {
            CachedIncludeValidation::RemoteUrl { .. }
            | CachedIncludeValidation::UnsupportedLocalPath { .. } => Ok(true),
            CachedIncludeValidation::MissingLocalPath { path } => {
                Ok(fs::metadata(path).is_err_and(|error| error.kind() == io::ErrorKind::NotFound))
            }
            CachedIncludeValidation::LocalFile {
                path,
                document_generation,
            } => {
                self.load_document(path.clone(), false)?;
                Ok(self.session.document_generation(path) == Some(*document_generation))
            }
            CachedIncludeValidation::LocalDirectory {
                path,
                discovered_paths,
            } => {
                let allowed_root = path
                    .parent()
                    .expect("directory include path should have a parent");
                let current_paths = collect_directory_include_paths(path, allowed_root)?;
                if current_paths.len() != discovered_paths.len() {
                    return Ok(false);
                }

                for (current_path, (expected_path, expected_generation)) in
                    current_paths.iter().zip(discovered_paths)
                {
                    if current_path != expected_path {
                        return Ok(false);
                    }

                    self.load_document(current_path.clone(), false)?;
                    if self.session.document_generation(current_path) != Some(*expected_generation)
                    {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
        }
    }
}

#[derive(Debug)]
struct ResolvedIncludeWork {
    include: ResolvedInclude,
    discovered_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct WorkspaceBaseDirective {
    raw_value: String,
    value_kind: DirectiveValueKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ConstantEnvironment {
    bindings: BTreeMap<String, String>,
}

impl ConstantEnvironment {
    fn insert(&mut self, name: String, value: String) {
        self.bindings.insert(name, value);
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.bindings.get(name).map(String::as_str)
    }

    fn context_key_entries(&self) -> Vec<(String, String)> {
        self.bindings
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }
}

#[derive(Debug, Clone)]
struct DocumentContext {
    key: DocumentContextKey,
    path: PathBuf,
    inherited_constants: ConstantEnvironment,
}

impl DocumentContext {
    fn new(path: PathBuf, inherited_constants: ConstantEnvironment) -> Self {
        let key = DocumentContextKey {
            path: path.clone(),
            inherited_constants: inherited_constants.context_key_entries(),
        };

        Self {
            key,
            path,
            inherited_constants,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DocumentContextKey {
    path: PathBuf,
    inherited_constants: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct ProcessedDocumentContext {
    exported_constants: ConstantEnvironment,
    workspace_base_context: Option<DocumentContextKey>,
    direct_includes: Vec<ResolvedInclude>,
    included_contexts: Vec<DocumentContextKey>,
}

enum DocumentDirectiveEvent<'a> {
    Constant(&'a ConstantDefinition),
    Include(&'a IncludeDirective),
}

impl DocumentDirectiveEvent<'_> {
    const fn sort_rank(&self) -> usize {
        match self {
            Self::Constant(_) => 0,
            Self::Include(_) => 1,
        }
    }

    const fn start_byte(&self) -> usize {
        match self {
            Self::Constant(constant) => constant.span.start_byte,
            Self::Include(include) => include.span.start_byte,
        }
    }
}

fn processed_context_dependency_keys(
    processed: &ProcessedDocumentContext,
) -> impl Iterator<Item = &DocumentContextKey> {
    processed
        .workspace_base_context
        .iter()
        .chain(processed.included_contexts.iter())
}

fn start_contexts(
    normalized_roots: &[PathBuf],
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
) -> Vec<DocumentContext> {
    let mut contexts = Vec::new();
    let mut seen = BTreeSet::new();

    for root in normalized_roots {
        for path in start_paths_for_root(root, loaded_documents) {
            let context = DocumentContext::new(path, ConstantEnvironment::default());
            if seen.insert(context.key.clone()) {
                contexts.push(context);
            }
        }
    }

    contexts.sort_by(|left, right| left.key.cmp(&right.key));
    contexts
}

fn start_paths_for_root(
    root: &Path,
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }

    let mut paths = documents_under_root(root, loaded_documents, |document| {
        document.kind() == WorkspaceDocumentKind::Entry
    });
    if paths.is_empty() {
        paths = documents_under_root(root, loaded_documents, |_| true);
    }
    paths
}

fn documents_under_root<F>(
    root: &Path,
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
    predicate: F,
) -> Vec<PathBuf>
where
    F: Fn(&WorkspaceDocument) -> bool,
{
    let mut paths = loaded_documents
        .iter()
        .filter_map(|(path, document)| {
            (path.starts_with(root) && predicate(document)).then_some(path.clone())
        })
        .collect::<Vec<_>>();

    paths.sort();
    paths.dedup();
    paths
}

fn document_directive_events<'a>(
    constant_definitions: &'a [ConstantDefinition],
    include_directives: &'a [IncludeDirective],
) -> Vec<DocumentDirectiveEvent<'a>> {
    let mut events = constant_definitions
        .iter()
        .map(DocumentDirectiveEvent::Constant)
        .chain(
            include_directives
                .iter()
                .map(DocumentDirectiveEvent::Include),
        )
        .collect::<Vec<_>>();

    events.sort_by(|left, right| {
        left.start_byte()
            .cmp(&right.start_byte())
            .then_with(|| left.sort_rank().cmp(&right.sort_rank()))
    });
    events
}

fn apply_constant_definition(
    constant: &ConstantDefinition,
    current_constants: &mut ConstantEnvironment,
) {
    let expanded_value = expand_string_substitutions(&constant.value, current_constants);
    current_constants.insert(constant.name.clone(), expanded_value);
}

fn normalize_existing_path(path: &Path) -> io::Result<PathBuf> {
    fs::canonicalize(path)
}

fn scan_workspace_root(root: &Path) -> io::Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut builder = WalkBuilder::new(root);
    builder.sort_by_file_path(std::cmp::Ord::cmp);

    let mut paths = Vec::new();

    for entry in builder.build() {
        let entry = entry.map_err(io::Error::other)?;
        let entry_path = entry.path();
        let is_file = entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file());

        if is_file && has_dsl_extension(entry_path) {
            paths.push(normalize_existing_path(entry_path)?);
        }
    }

    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn has_dsl_extension(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dsl"))
}

fn document_id_from_path(path: &Path) -> DocumentId {
    DocumentId::new(path.to_string_lossy().into_owned())
}

fn resolve_include(
    including_document: &DocumentId,
    including_document_path: &Path,
    directive: &IncludeDirective,
    constants: &ConstantEnvironment,
) -> io::Result<ResolvedIncludeWork> {
    let target_text = expand_string_substitutions(&normalized_include_value(directive), constants);
    let base_include = |target: WorkspaceIncludeTarget, discovered_paths: Vec<PathBuf>| {
        let discovered_documents = discovered_paths
            .iter()
            .map(|path| document_id_from_path(path))
            .collect();

        ResolvedIncludeWork {
            include: ResolvedInclude {
                including_document: including_document.clone(),
                raw_value: directive.raw_value.clone(),
                target_text: target_text.clone(),
                span: directive.span,
                value_span: directive.value_span,
                target,
                discovered_documents,
            },
            discovered_paths,
        }
    };

    if is_remote_include(&target_text) {
        return Ok(base_include(
            WorkspaceIncludeTarget::RemoteUrl {
                url: target_text.clone(),
            },
            Vec::new(),
        ));
    }

    let Some(parent_directory) = including_document_path.parent() else {
        return Err(io::Error::other(format!(
            "document path has no parent directory: {}",
            including_document_path.display()
        )));
    };
    let canonical_parent_directory = normalize_existing_path(parent_directory)?;
    let relative_target = PathBuf::from(&target_text);
    let joined_target = parent_directory.join(&relative_target);

    if !is_supported_local_include_path(&relative_target) {
        return Ok(base_include(
            WorkspaceIncludeTarget::UnsupportedLocalPath {
                path: joined_target,
            },
            Vec::new(),
        ));
    }

    match fs::metadata(&joined_target) {
        Ok(metadata) if metadata.is_file() => {
            let canonical_file = normalize_existing_path(&joined_target)?;

            if !canonical_file.starts_with(&canonical_parent_directory) {
                return Ok(base_include(
                    WorkspaceIncludeTarget::UnsupportedLocalPath {
                        path: canonical_file,
                    },
                    Vec::new(),
                ));
            }

            Ok(base_include(
                WorkspaceIncludeTarget::LocalFile {
                    path: canonical_file.clone(),
                },
                vec![canonical_file],
            ))
        }
        Ok(metadata) if metadata.is_dir() => {
            let canonical_directory = normalize_existing_path(&joined_target)?;

            if !canonical_directory.starts_with(&canonical_parent_directory) {
                return Ok(base_include(
                    WorkspaceIncludeTarget::UnsupportedLocalPath {
                        path: canonical_directory,
                    },
                    Vec::new(),
                ));
            }

            let discovered_paths =
                collect_directory_include_paths(&canonical_directory, &canonical_parent_directory)?;

            Ok(base_include(
                WorkspaceIncludeTarget::LocalDirectory {
                    path: canonical_directory,
                },
                discovered_paths,
            ))
        }
        Ok(_) => Ok(base_include(
            WorkspaceIncludeTarget::UnsupportedLocalPath {
                path: joined_target,
            },
            Vec::new(),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(base_include(
            WorkspaceIncludeTarget::MissingLocalPath {
                path: joined_target,
            },
            Vec::new(),
        )),
        Err(error) => Err(error),
    }
}

fn normalized_include_value(directive: &IncludeDirective) -> String {
    normalized_directive_value(&directive.raw_value, &directive.value_kind)
}

fn workspace_base_directive(snapshot: &crate::DocumentSnapshot) -> Option<WorkspaceBaseDirective> {
    let root = snapshot.tree().root_node();
    let mut cursor = root.walk();
    let workspace = root
        .named_children(&mut cursor)
        .find(|child| child.kind() == "workspace")?;
    let base = workspace.child_by_field_name("base")?;

    Some(WorkspaceBaseDirective {
        raw_value: snapshot.source()[base.byte_range()].to_owned(),
        value_kind: DirectiveValueKind::from_node_kind(base.kind()),
    })
}

fn resolve_workspace_base(
    workspace_path: &Path,
    workspace_base: &WorkspaceBaseDirective,
    constants: &ConstantEnvironment,
) -> io::Result<PathBuf> {
    let base_text = expand_string_substitutions(
        &normalized_directive_value(&workspace_base.raw_value, &workspace_base.value_kind),
        constants,
    );
    if is_remote_include(&base_text) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("remote workspace bases are not supported: {base_text}"),
        ));
    }

    let parent = workspace_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "workspace entry has no parent directory for base resolution: {}",
                workspace_path.display()
            ),
        )
    })?;
    let canonical_parent_directory = normalize_existing_path(parent)?;
    let relative_target = PathBuf::from(&base_text);
    let base_path = parent.join(&relative_target);
    if !is_supported_local_include_path(&relative_target) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "workspace base path escapes the allowed subtree: {}",
                base_path.display()
            ),
        ));
    }
    let metadata = fs::metadata(&base_path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("workspace base does not exist: {}", base_path.display()),
            )
        } else {
            error
        }
    })?;

    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "workspace base must resolve to a file: {}",
                base_path.display()
            ),
        ));
    }

    let canonical_base = normalize_existing_path(&base_path)?;
    if !canonical_base.starts_with(&canonical_parent_directory) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "workspace base path escapes the allowed subtree: {}",
                canonical_base.display()
            ),
        ));
    }

    Ok(canonical_base)
}

fn expand_string_substitutions(value: &str, constants: &ConstantEnvironment) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut remaining = value;

    while let Some(marker_start) = remaining.find("${") {
        expanded.push_str(&remaining[..marker_start]);

        let placeholder = &remaining[marker_start..];
        let Some(placeholder_end) = placeholder.find('}') else {
            expanded.push_str(placeholder);
            return expanded;
        };

        let name = &placeholder[2..placeholder_end];
        if is_supported_substitution_name(name) {
            if let Some(replacement) = constants.get(name) {
                expanded.push_str(replacement);
            } else {
                expanded.push_str(&placeholder[..=placeholder_end]);
            }
        } else {
            expanded.push_str(&placeholder[..=placeholder_end]);
        }

        remaining = &placeholder[placeholder_end + 1..];
    }

    expanded.push_str(remaining);
    expanded
}

fn is_supported_substitution_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
}

fn is_remote_include(target_text: &str) -> bool {
    target_text.starts_with("https://")
}

fn is_supported_local_include_path(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn resolve_local_resource_path(document: &DocumentId, value: &ValueFact) -> Option<PathBuf> {
    if value.value_kind == DirectiveValueKind::TextBlockString
        || value.normalized_text.contains("${")
        || is_remote_resource_value(&value.normalized_text)
    {
        return None;
    }

    let relative_path = PathBuf::from(&value.normalized_text);
    if !is_supported_local_include_path(&relative_path) {
        return None;
    }

    // TODO: Expand `${...}` substitutions here once workspace semantic packets
    // carry the effective constant environment for each processed document.
    let document_path = Path::new(document.as_str());
    let parent_directory = document_path.parent()?;
    Some(parent_directory.join(relative_path))
}

fn is_remote_resource_value(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://") || value.starts_with("data:")
}

fn collect_directory_include_paths(
    directory: &Path,
    allowed_root: &Path,
) -> io::Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(directory);
    builder.hidden(false);
    builder.ignore(false);
    builder.git_ignore(false);
    builder.git_global(false);
    builder.git_exclude(false);
    builder.sort_by_file_path(std::cmp::Ord::cmp);

    let mut paths = Vec::new();

    for entry in builder.build() {
        let entry = entry.map_err(io::Error::other)?;
        let entry_path = entry.path();
        let is_file = entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file());

        if !is_file {
            continue;
        }

        let canonical_path = normalize_existing_path(entry_path)?;
        if canonical_path.starts_with(allowed_root) {
            paths.push(canonical_path);
        }
    }

    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn include_diagnostics(resolved_includes: &[ResolvedInclude]) -> Vec<RuledDiagnostic> {
    let cycle_include_indices = cycle_include_indices(resolved_includes);
    let mut diagnostics = Vec::new();

    for (index, include) in resolved_includes.iter().enumerate() {
        match include.target() {
            WorkspaceIncludeTarget::MissingLocalPath { .. } => {
                diagnostics.push(RuledDiagnostic::missing_local_target(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::UnsupportedLocalPath { .. } => {
                diagnostics.push(RuledDiagnostic::escapes_allowed_subtree(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::RemoteUrl { .. } => {
                diagnostics.push(RuledDiagnostic::unsupported_remote_target(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::LocalFile { .. }
            | WorkspaceIncludeTarget::LocalDirectory { .. } => {}
        }

        if cycle_include_indices.contains(&index) {
            diagnostics.push(RuledDiagnostic::include_cycle(
                include.including_document(),
                include.target_text(),
                include.span(),
                include.value_span(),
            ));
        }
    }

    diagnostics.sort_by(|left, right| {
        left.document()
            .cmp(&right.document())
            .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
            .then_with(|| left.rule.cmp(&right.rule))
            .then_with(|| left.target_text().cmp(&right.target_text()))
    });
    diagnostics
}

fn cycle_include_indices(resolved_includes: &[ResolvedInclude]) -> BTreeSet<usize> {
    let mut adjacency = BTreeMap::<DocumentId, Vec<(usize, DocumentId)>>::new();

    for (index, include) in resolved_includes.iter().enumerate() {
        for discovered_document in include.discovered_documents() {
            adjacency
                .entry(include.including_document().clone())
                .or_default()
                .push((index, discovered_document.clone()));
        }
    }

    let mut cycle_indices = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack_documents = Vec::new();
    let mut stack_edges = Vec::new();

    for document in adjacency.keys() {
        collect_cycle_include_indices(
            document,
            &adjacency,
            &mut visited,
            &mut stack_documents,
            &mut stack_edges,
            &mut cycle_indices,
        );
    }

    cycle_indices
}

fn collect_cycle_include_indices(
    document: &DocumentId,
    adjacency: &BTreeMap<DocumentId, Vec<(usize, DocumentId)>>,
    visited: &mut BTreeSet<DocumentId>,
    stack_documents: &mut Vec<DocumentId>,
    stack_edges: &mut Vec<usize>,
    cycle_indices: &mut BTreeSet<usize>,
) {
    if visited.contains(document) {
        return;
    }

    stack_documents.push(document.clone());

    if let Some(edges) = adjacency.get(document) {
        for (edge_index, child) in edges {
            if let Some(stack_index) = stack_documents
                .iter()
                .position(|candidate| candidate == child)
            {
                for cycle_edge in stack_edges.iter().skip(stack_index) {
                    cycle_indices.insert(*cycle_edge);
                }
                cycle_indices.insert(*edge_index);
                continue;
            }

            if visited.contains(child) {
                continue;
            }

            stack_edges.push(*edge_index);
            collect_cycle_include_indices(
                child,
                adjacency,
                visited,
                stack_documents,
                stack_edges,
                cycle_indices,
            );
            let _ = stack_edges.pop();
        }
    }

    let _ = stack_documents.pop();
    visited.insert(document.clone());
}

/// Effective element-identifier mode for one document inside one workspace instance.
///
/// This folds together the document-local `!identifiers` directives and any
/// inherited workspace-level mode so downstream consumers do not need to
/// re-derive the same policy from raw directive facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementIdentifierMode {
    /// Element bindings resolve through their flat binding names.
    ///
    /// Example: with `!identifiers flat`, a container declared as
    /// `api = container "API"` is referenced as `api`.
    Flat,
    /// Element bindings resolve through canonical hierarchical keys.
    ///
    /// Example: with `!identifiers hierarchical`, a container declared as
    /// `api = container "API"` inside `softwareSystem1 = softwareSystem "System 1"`
    /// is referenced as `softwareSystem1.api`.
    Hierarchical,
    /// Element bindings stay intentionally deferred because the effective mode is
    /// unsupported for the bounded semantic surface.
    ///
    /// Example: `!identifiers custom` is parsed as an unrecognized mode, so the
    /// workspace index records the document as deferred instead of guessing how
    /// a reference such as `api` should resolve.
    Deferred,
}

fn build_workspace_indexes(
    session: &mut WorkspaceAnalysisSession,
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
    start_contexts: &[DocumentContext],
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
) -> Vec<WorkspaceIndex> {
    let documents_by_id = loaded_documents
        .values()
        .map(|document| (document.id().clone(), document))
        .collect::<BTreeMap<_, _>>();

    start_contexts
        .iter()
        .enumerate()
        .map(|(ordinal, start_context)| {
            let instance_id = WorkspaceInstanceId(ordinal);
            build_workspace_index(
                session,
                instance_id,
                start_context,
                processed_contexts,
                &documents_by_id,
            )
        })
        .collect()
}

fn build_document_instances(
    workspace_indexes: &[WorkspaceIndex],
) -> BTreeMap<DocumentId, Vec<WorkspaceInstanceId>> {
    let mut document_instances = BTreeMap::<DocumentId, Vec<WorkspaceInstanceId>>::new();

    for workspace_index in workspace_indexes {
        for document in workspace_index.documents() {
            document_instances
                .entry(document.clone())
                .or_default()
                .push(workspace_index.id());
        }
    }

    document_instances
}

fn build_workspace_index(
    session: &mut WorkspaceAnalysisSession,
    instance_id: WorkspaceInstanceId,
    start_context: &DocumentContext,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
) -> WorkspaceIndex {
    let _root_context_revision = session
        .processed_context_revision(&start_context.key)
        .expect("BUG: start context should be processed before building indexes");

    // Build two slices from the same processed-context tree:
    //
    // 1. the full assembled instance, which includes extended bases because
    //    binding/reference resolution needs the final inherited symbol table;
    // 2. the narrower DSL definition, which intentionally excludes bases so
    //    structural rules like repeated `model` / `views` sections talk about
    //    one definition rather than one definition plus its parent workspace.
    let instance_documents = collect_documents_for_context(
        &start_context.key,
        processed_contexts,
        ContextDocumentCollection::Instance,
    );
    let definition_documents = collect_documents_for_context(
        &start_context.key,
        processed_contexts,
        ContextDocumentCollection::Definition,
    );
    let inherited_workspace_modes = inherited_workspace_modes_for_context(
        &start_context.key,
        processed_contexts,
        documents_by_id,
    );
    let document_semantic_generations = instance_documents
        .iter()
        .map(|document_id| {
            let document = documents_by_id
                .get(document_id)
                .expect("BUG: workspace-index document should exist");
            (document_id.clone(), document.semantic_generation())
        })
        .collect::<Vec<_>>();

    if let Some(cached) = session.cached_workspace_instance(&start_context.key)
        && cached.document_semantic_generations == document_semantic_generations
    {
        return cached.workspace_index(instance_id);
    }

    let root_document = document_id_from_path(&start_context.path);
    let root_document = documents_by_id
        .get(&root_document)
        .expect("BUG: workspace-index root document should exist");
    let instance_semantic_documents = instance_documents
        .iter()
        .map(|document_id| {
            documents_by_id
                .get(document_id)
                .expect("BUG: workspace-index document should exist")
                .semantic_facts()
        })
        .collect::<Vec<_>>();
    let definition_semantic_documents = definition_documents
        .iter()
        .map(|document_id| {
            documents_by_id
                .get(document_id)
                .expect("BUG: workspace-index definition document should exist")
                .semantic_facts()
        })
        .collect::<Vec<_>>();
    let derived = Arc::new(build_derived_workspace_instance(
        root_document.semantic_facts(),
        &instance_semantic_documents,
        &definition_semantic_documents,
        &inherited_workspace_modes,
    ));
    session.store_workspace_instance(
        &start_context.key,
        document_semantic_generations,
        Arc::clone(&derived),
    );
    WorkspaceIndex::from_derived(instance_id, derived)
}

fn workspace_facts_assembly_key(
    session: &WorkspaceAnalysisSession,
    start_contexts: &[DocumentContext],
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
) -> WorkspaceFactsAssemblyKey {
    let workspace_instances = start_contexts
        .iter()
        .map(|start_context| RevisionedContextKey {
            context_key: start_context.key.clone(),
            revision: session
                .processed_context_revision(&start_context.key)
                .expect("BUG: start context should be processed before assembly"),
        })
        .collect();

    let processed_contexts = processed_contexts
        .keys()
        .map(|context_key| RevisionedContextKey {
            context_key: context_key.clone(),
            revision: session
                .processed_context_revision(context_key)
                .expect("BUG: materialized processed context should have a cached revision"),
        })
        .collect();

    WorkspaceFactsAssemblyKey {
        processed_contexts,
        workspace_instances,
    }
}

fn build_workspace_facts_assembly(
    session: &mut WorkspaceAnalysisSession,
    assembly_key: WorkspaceFactsAssemblyKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    workspace_indexes: &[WorkspaceIndex],
) -> Arc<DerivedWorkspaceFactsAssembly> {
    if let Some(cached) = session.cached_workspace_facts_assembly(&assembly_key) {
        return cached;
    }

    let mut resolved_includes = processed_contexts
        .values()
        .flat_map(|context| context.direct_includes.iter().cloned())
        .collect::<Vec<_>>();
    resolved_includes.sort_by(|left, right| {
        left.including_document()
            .as_str()
            .cmp(right.including_document().as_str())
            .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
            .then_with(|| left.target_text().cmp(right.target_text()))
    });

    let include_diagnostics = include_diagnostics(&resolved_includes);
    let document_instances = build_document_instances(workspace_indexes);
    let semantic_diagnostics = merge_semantic_diagnostics(workspace_indexes, &document_instances);

    let assembly = Arc::new(DerivedWorkspaceFactsAssembly {
        resolved_includes,
        include_diagnostics,
        document_instances,
        semantic_diagnostics,
    });
    session.store_workspace_facts_assembly(assembly_key, Arc::clone(&assembly));
    assembly
}

fn build_derived_workspace_instance(
    root_document: &WorkspaceSemanticDocumentFacts,
    documents: &[&WorkspaceSemanticDocumentFacts],
    definition_documents: &[&WorkspaceSemanticDocumentFacts],
    inherited_workspace_modes: &BTreeMap<DocumentId, Option<IdentifierMode>>,
) -> DerivedWorkspaceInstance {
    // `definition_documents` exists purely to scope structural workspace rules to
    // one assembled definition. We intentionally keep only the full instance
    // document list on the derived payload because downstream callers reason
    // about the assembled semantic surface, not the narrower structural slice.
    let _ = root_document;
    let bindings = build_binding_tables(documents, inherited_workspace_modes);
    let mut semantic_diagnostics = bindings.semantic_diagnostics.clone();
    semantic_diagnostics.extend(workspace_structure_diagnostics(definition_documents));
    semantic_diagnostics.extend(workspace_scope_diagnostics(definition_documents, documents));
    semantic_diagnostics.extend(element_selector_diagnostics(documents, &bindings));

    let reference_tables = build_reference_resolution_tables(documents, &bindings);
    let documents_by_id = documents
        .iter()
        .map(|document| (document.document_id.clone(), *document))
        .collect::<BTreeMap<_, _>>();
    semantic_diagnostics.extend(deployment_semantic_diagnostics(
        documents,
        &documents_by_id,
        &reference_tables,
    ));
    semantic_diagnostics.extend(resource_semantic_diagnostics(documents));
    semantic_diagnostics.extend(view_semantic_diagnostics(
        documents,
        &documents_by_id,
        &bindings,
        &reference_tables,
    ));
    semantic_diagnostics.extend(reference_tables.semantic_diagnostics);

    let mut references_by_target = reference_tables.references_by_target;
    for references in references_by_target.values_mut() {
        references.sort();
        references.dedup();
    }
    sort_semantic_diagnostics(&mut semantic_diagnostics);

    DerivedWorkspaceInstance {
        root_document: root_document.document_id.clone(),
        documents: documents
            .iter()
            .map(|document| document.document_id.clone())
            .collect(),
        element_identifier_modes: bindings.element_modes,
        unique_element_bindings: bindings.unique_elements,
        duplicate_element_bindings: bindings.duplicate_elements,
        unique_deployment_bindings: bindings.unique_deployments,
        duplicate_deployment_bindings: bindings.duplicate_deployments,
        unique_relationship_bindings: bindings.unique_relationships,
        duplicate_relationship_bindings: bindings.duplicate_relationships,
        reference_resolutions: reference_tables.resolutions,
        references_by_target,
        semantic_diagnostics,
    }
}

/// The same processed-context tree feeds two different semantic projections.
///
/// `Instance` walks the complete assembled workspace, including extended bases,
/// because binding/reference resolution needs the final inherited symbol table.
/// `Definition` stops at ordinary includes so structural rules can talk about
/// one DSL definition without treating its extended base as a repeated sibling.
#[derive(Clone, Copy)]
enum ContextDocumentCollection {
    Instance,
    Definition,
}

fn collect_documents_for_context(
    context_key: &DocumentContextKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    collection: ContextDocumentCollection,
) -> Vec<DocumentId> {
    let mut visited_contexts = BTreeSet::new();
    let mut seen_documents = BTreeSet::new();
    let mut collected_documents = Vec::new();
    collect_context_documents(
        context_key,
        processed_contexts,
        collection,
        &mut visited_contexts,
        &mut seen_documents,
        &mut collected_documents,
    );
    collected_documents
}

fn collect_context_documents(
    context_key: &DocumentContextKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    collection: ContextDocumentCollection,
    visited_contexts: &mut BTreeSet<DocumentContextKey>,
    seen_documents: &mut BTreeSet<DocumentId>,
    collected_documents: &mut Vec<DocumentId>,
) {
    if !visited_contexts.insert(context_key.clone()) {
        return;
    }

    let document_id = document_id_from_path(&context_key.path);
    if seen_documents.insert(document_id.clone()) {
        collected_documents.push(document_id);
    }

    let Some(processed_context) = processed_contexts.get(context_key) else {
        return;
    };

    match collection {
        ContextDocumentCollection::Instance => {
            for child_context in processed_context_dependency_keys(processed_context) {
                collect_context_documents(
                    child_context,
                    processed_contexts,
                    collection,
                    visited_contexts,
                    seen_documents,
                    collected_documents,
                );
            }
        }
        ContextDocumentCollection::Definition => {
            for child_context in &processed_context.included_contexts {
                collect_context_documents(
                    child_context,
                    processed_contexts,
                    collection,
                    visited_contexts,
                    seen_documents,
                    collected_documents,
                );
            }
        }
    }
}

fn inherited_workspace_modes_for_context(
    context_key: &DocumentContextKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
) -> BTreeMap<DocumentId, Option<IdentifierMode>> {
    let mut visited_contexts = BTreeSet::new();
    let mut inherited_modes = BTreeMap::new();
    collect_inherited_workspace_modes(
        context_key,
        None,
        processed_contexts,
        documents_by_id,
        &mut visited_contexts,
        &mut inherited_modes,
    );
    inherited_modes
}

fn collect_inherited_workspace_modes(
    context_key: &DocumentContextKey,
    inherited_workspace_mode: Option<IdentifierMode>,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
    visited_contexts: &mut BTreeSet<DocumentContextKey>,
    inherited_modes: &mut BTreeMap<DocumentId, Option<IdentifierMode>>,
) {
    if !visited_contexts.insert(context_key.clone()) {
        return;
    }

    let document_id = document_id_from_path(&context_key.path);
    if let Some(existing_mode) = inherited_modes.get(&document_id) {
        debug_assert_eq!(existing_mode, &inherited_workspace_mode);
    } else {
        inherited_modes.insert(document_id.clone(), inherited_workspace_mode.clone());
    }

    let Some(document) = documents_by_id.get(&document_id) else {
        return;
    };
    let next_workspace_mode =
        document_workspace_identifier_mode(&document.semantic_facts().identifier_modes)
            .or(inherited_workspace_mode);
    let Some(processed_context) = processed_contexts.get(context_key) else {
        return;
    };

    for child_context in processed_context_dependency_keys(processed_context) {
        collect_inherited_workspace_modes(
            child_context,
            next_workspace_mode.clone(),
            processed_contexts,
            documents_by_id,
            visited_contexts,
            inherited_modes,
        );
    }
}

struct WorkspaceBindingTables {
    element_modes: BTreeMap<DocumentId, ElementIdentifierMode>,
    unique_elements: BTreeMap<String, SymbolHandle>,
    duplicate_elements: BTreeMap<String, Vec<SymbolHandle>>,
    unique_deployments: BTreeMap<String, SymbolHandle>,
    duplicate_deployments: BTreeMap<String, Vec<SymbolHandle>>,
    unique_relationships: BTreeMap<String, SymbolHandle>,
    duplicate_relationships: BTreeMap<String, Vec<SymbolHandle>>,
    semantic_diagnostics: Vec<RuledDiagnostic>,
}

fn build_binding_tables(
    documents: &[&WorkspaceSemanticDocumentFacts],
    inherited_workspace_modes: &BTreeMap<DocumentId, Option<IdentifierMode>>,
) -> WorkspaceBindingTables {
    let mut element_modes = BTreeMap::<DocumentId, ElementIdentifierMode>::new();
    let mut element_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();
    let mut deployment_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();
    let mut relationship_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();

    for document in documents {
        let inherited_workspace_mode = inherited_workspace_modes
            .get(&document.document_id)
            .and_then(|mode| mode.as_ref());
        let element_mode = effective_element_identifier_mode(document, inherited_workspace_mode);
        element_modes.insert(document.document_id.clone(), element_mode);

        for symbol in &document.symbols {
            let Some(binding_name) = symbol.binding_name.as_deref() else {
                continue;
            };

            let handle = SymbolHandle {
                document: document.document_id.clone(),
                symbol_id: symbol.id,
            };

            match symbol.kind {
                SymbolKind::Relationship => {
                    relationship_bindings
                        .entry(binding_name.to_owned())
                        .or_default()
                        .push(handle);
                }
                SymbolKind::Person
                | SymbolKind::SoftwareSystem
                | SymbolKind::Container
                | SymbolKind::Component => {
                    let Some(binding_key) = canonical_binding_key(
                        &document.symbols,
                        symbol.id,
                        element_mode,
                        CanonicalBindingKind::Element,
                    ) else {
                        continue;
                    };

                    element_bindings
                        .entry(binding_key)
                        .or_default()
                        .push(handle);
                }
                SymbolKind::DeploymentEnvironment
                | SymbolKind::DeploymentNode
                | SymbolKind::InfrastructureNode
                | SymbolKind::ContainerInstance
                | SymbolKind::SoftwareSystemInstance => {
                    let Some(binding_key) = canonical_binding_key(
                        &document.symbols,
                        symbol.id,
                        element_mode,
                        CanonicalBindingKind::Deployment,
                    ) else {
                        continue;
                    };

                    deployment_bindings
                        .entry(binding_key)
                        .or_default()
                        .push(handle);
                }
            }
        }
    }

    let (unique_element_bindings, duplicate_element_bindings) =
        split_binding_table(element_bindings);
    let (unique_deployment_bindings, duplicate_deployment_bindings) =
        split_binding_table(deployment_bindings);
    let (unique_relationship_bindings, duplicate_relationship_bindings) =
        split_binding_table(relationship_bindings);

    let mut semantic_diagnostics = Vec::new();
    push_duplicate_binding_diagnostics(
        "element",
        &duplicate_element_bindings,
        documents,
        &mut semantic_diagnostics,
    );
    push_duplicate_binding_diagnostics(
        "deployment",
        &duplicate_deployment_bindings,
        documents,
        &mut semantic_diagnostics,
    );
    push_duplicate_binding_diagnostics(
        "relationship",
        &duplicate_relationship_bindings,
        documents,
        &mut semantic_diagnostics,
    );

    WorkspaceBindingTables {
        element_modes,
        unique_elements: unique_element_bindings,
        duplicate_elements: duplicate_element_bindings,
        unique_deployments: unique_deployment_bindings,
        duplicate_deployments: duplicate_deployment_bindings,
        unique_relationships: unique_relationship_bindings,
        duplicate_relationships: duplicate_relationship_bindings,
        semantic_diagnostics,
    }
}

struct WorkspaceReferenceTables {
    resolutions: BTreeMap<ReferenceHandle, ReferenceResolutionStatus>,
    // Later semantic passes ask "what symbol does this exact `(kind, span)` site
    // resolve to?" far more often than they ask for the raw resolution enum.
    // Cache that direct lookup once so view/deployment rules do not repeatedly
    // rescan `document.references` just to rediscover the same target handle.
    resolved_targets: BTreeMap<DocumentId, BTreeMap<(u8, TextSpan), SymbolHandle>>,
    references_by_target: BTreeMap<SymbolHandle, Vec<ReferenceHandle>>,
    semantic_diagnostics: Vec<RuledDiagnostic>,
}

fn build_reference_resolution_tables(
    documents: &[&WorkspaceSemanticDocumentFacts],
    bindings: &WorkspaceBindingTables,
) -> WorkspaceReferenceTables {
    let mut reference_resolutions = BTreeMap::<ReferenceHandle, ReferenceResolutionStatus>::new();
    let mut resolved_targets =
        BTreeMap::<DocumentId, BTreeMap<(u8, TextSpan), SymbolHandle>>::new();
    let mut references_by_target = BTreeMap::<SymbolHandle, Vec<ReferenceHandle>>::new();
    let mut semantic_diagnostics = Vec::new();

    for document in documents {
        for (reference_index, reference) in document.references.iter().enumerate() {
            let handle = ReferenceHandle {
                document: document.document_id.clone(),
                reference_index,
            };
            let status = resolve_reference_status(document, reference, bindings);

            if !document.has_syntax_errors {
                match status {
                    ReferenceResolutionStatus::UnresolvedNoMatch => {
                        semantic_diagnostics.push(RuledDiagnostic::unresolved_reference(
                            &document.document_id,
                            &reference.raw_text,
                            reference.span,
                        ));
                    }
                    ReferenceResolutionStatus::AmbiguousDuplicateBinding
                    | ReferenceResolutionStatus::AmbiguousElementVsRelationship => {
                        semantic_diagnostics.push(RuledDiagnostic::ambiguous_reference(
                            &document.document_id,
                            &reference.raw_text,
                            reference.span,
                        ));
                    }
                    ReferenceResolutionStatus::Resolved(_)
                    | ReferenceResolutionStatus::DeferredByScopePolicy => {}
                }
            }

            if let ReferenceResolutionStatus::Resolved(target) = &status {
                resolved_targets
                    .entry(document.document_id.clone())
                    .or_default()
                    .insert(reference_lookup_key(reference.kind, reference.span), target.clone());
                references_by_target
                    .entry(target.clone())
                    .or_default()
                    .push(handle.clone());
            }

            reference_resolutions.insert(handle, status);
        }
    }

    WorkspaceReferenceTables {
        resolutions: reference_resolutions,
        resolved_targets,
        references_by_target,
        semantic_diagnostics,
    }
}

fn split_binding_table(
    bindings: BTreeMap<String, Vec<SymbolHandle>>,
) -> (
    BTreeMap<String, SymbolHandle>,
    BTreeMap<String, Vec<SymbolHandle>>,
) {
    let mut unique = BTreeMap::new();
    let mut duplicates = BTreeMap::new();

    for (key, mut handles) in bindings {
        handles.sort();
        handles.dedup();

        if let [handle] = handles.as_slice() {
            unique.insert(key, handle.clone());
        } else {
            duplicates.insert(key, handles);
        }
    }

    (unique, duplicates)
}

fn push_duplicate_binding_diagnostics(
    binding_kind: &str,
    duplicate_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
    documents: &[&WorkspaceSemanticDocumentFacts],
    diagnostics: &mut Vec<RuledDiagnostic>,
) {
    let documents_by_id = documents
        .iter()
        .map(|document| (&document.document_id, *document))
        .collect::<BTreeMap<_, _>>();

    for (key, handles) in duplicate_bindings {
        for handle in handles {
            let document = documents_by_id
                .get(handle.document())
                .expect("BUG: duplicate-binding document should exist");
            if document.has_syntax_errors {
                continue;
            }

            let symbol = document
                .symbols
                .get(handle.symbol_id().0)
                .expect("BUG: duplicate-binding symbol should exist");
            diagnostics.push(RuledDiagnostic::duplicate_binding(
                handle.document(),
                binding_kind,
                key,
                symbol.span,
            ));
        }
    }
}

fn workspace_structure_diagnostics(
    definition_documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();
    push_repeated_workspace_section_diagnostics(
        WorkspaceSectionKind::Model,
        "model",
        definition_documents,
        &mut diagnostics,
    );
    push_repeated_workspace_section_diagnostics(
        WorkspaceSectionKind::Views,
        "views",
        definition_documents,
        &mut diagnostics,
    );
    diagnostics
}

fn push_repeated_workspace_section_diagnostics(
    section_kind: WorkspaceSectionKind,
    section_name: &str,
    documents: &[&WorkspaceSemanticDocumentFacts],
    diagnostics: &mut Vec<RuledDiagnostic>,
) {
    let occurrences = documents
        .iter()
        .filter(|document| !document.has_syntax_errors)
        .flat_map(|document| {
            document
                .workspace_sections
                .iter()
                .filter(move |fact| fact.kind == section_kind)
                .map(move |fact| (document.document_id.clone(), fact.span))
        })
        .collect::<Vec<_>>();
    let Some((first_document, first_span)) = occurrences.first().cloned() else {
        return;
    };

    for (document, span) in occurrences.into_iter().skip(1) {
        let mut diagnostic =
            RuledDiagnostic::repeated_workspace_section(&document, section_name, span);
        let annotation = if document == first_document {
            Annotation::secondary(first_span)
        } else {
            Annotation::secondary(first_span).in_document(&first_document)
        }
        .message(format!("first {section_name} section here"));
        diagnostic.annotate(annotation);
        diagnostics.push(diagnostic);
    }
}

fn workspace_scope_diagnostics(
    definition_documents: &[&WorkspaceSemanticDocumentFacts],
    instance_documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let Some((scope_document, scope_fact)) = effective_workspace_scope(definition_documents)
        .or_else(|| effective_workspace_scope(instance_documents))
    else {
        return Vec::new();
    };
    let violations = workspace_scope_violations(&scope_fact.scope, instance_documents);
    if violations.is_empty() {
        return Vec::new();
    }

    violations
        .into_iter()
        .map(|violation| {
            let message = format!(
                "workspace is {} scoped, but the {} named {} has {}",
                workspace_scope_label(&scope_fact.scope),
                scope_violation_owner_label(&violation.owner),
                violation.owner.display_name,
                violation.child_plural,
            );
            let mut diagnostic =
                RuledDiagnostic::workspace_scope_mismatch(&scope_document, message, scope_fact.span);
            let annotation = if scope_document == violation.document {
                Annotation::secondary(violation.owner.span)
            } else {
                Annotation::secondary(violation.owner.span).in_document(&violation.document)
            }
            .message(format!(
                "{} named {} has {}",
                scope_violation_owner_label(&violation.owner),
                violation.owner.display_name,
                violation.child_plural,
            ));
            diagnostic.annotate(annotation);
            diagnostic
        })
        .collect()
}

fn effective_workspace_scope(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Option<(DocumentId, ConfigurationScopeFact)> {
    // Definition documents arrive in root-first include order. Prefer the first
    // scope we encounter so an included fragment cannot silently override the
    // root workspace entry's explicit scope declaration.
    documents.iter().find_map(|document| {
        document
            .configuration_scopes
            .last()
            .cloned()
            .map(|fact| (document.document_id.clone(), fact))
    })
}

#[derive(Debug)]
struct WorkspaceScopeViolation {
    document: DocumentId,
    owner: Symbol,
    child_plural: &'static str,
}

fn workspace_scope_violations(
    scope: &WorkspaceScope,
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<WorkspaceScopeViolation> {
    match scope {
        WorkspaceScope::Landscape => {
            scope_violations_for_child_kind(documents, SymbolKind::Container, "containers")
        }
        WorkspaceScope::SoftwareSystem => {
            scope_violations_for_child_kind(documents, SymbolKind::Component, "components")
        }
        WorkspaceScope::Container | WorkspaceScope::Component | WorkspaceScope::Other(_) => Vec::new(),
    }
}

fn scope_violations_for_child_kind(
    documents: &[&WorkspaceSemanticDocumentFacts],
    child_kind: SymbolKind,
    child_plural: &'static str,
) -> Vec<WorkspaceScopeViolation> {
    let mut seen_owners = BTreeSet::new();
    let mut violations = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for symbol in &document.symbols {
            if symbol.kind != child_kind {
                continue;
            }

            let Some(owner) = scope_violation_owner(&document.symbols, symbol.parent).cloned()
            else {
                continue;
            };
            if seen_owners.insert((document.document_id.clone(), owner.id)) {
                violations.push(WorkspaceScopeViolation {
                    document: document.document_id.clone(),
                    owner,
                    child_plural,
                });
            }
        }
    }

    violations
}

fn scope_violation_owner(symbols: &[Symbol], mut parent: Option<SymbolId>) -> Option<&Symbol> {
    while let Some(parent_id) = parent {
        let owner = symbols.get(parent_id.0)?;
        match owner.kind {
            SymbolKind::SoftwareSystem | SymbolKind::Container => return Some(owner),
            _ => parent = owner.parent,
        }
    }

    None
}

const fn scope_violation_owner_label(symbol: &Symbol) -> &'static str {
    match symbol.kind {
        SymbolKind::SoftwareSystem => "software system",
        SymbolKind::Container => "container",
        _ => "element",
    }
}

const fn workspace_scope_label(scope: &WorkspaceScope) -> &str {
    match scope {
        WorkspaceScope::Landscape => "landscape",
        WorkspaceScope::SoftwareSystem => "software system",
        WorkspaceScope::Container => "container",
        WorkspaceScope::Component => "component",
        WorkspaceScope::Other(raw) => raw.as_str(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ViewLocation {
    document: DocumentId,
    view_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RelationshipLocation {
    document: DocumentId,
    span: TextSpan,
}

impl RelationshipLocation {
    fn from_relationship(relationship: &DeclaredRelationship) -> Self {
        Self {
            document: relationship.document.clone(),
            span: relationship.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredRelationship {
    handle: Option<SymbolHandle>,
    document: DocumentId,
    span: TextSpan,
    source: SymbolHandle,
    destination: SymbolHandle,
    technology: Option<String>,
}

// The dynamic-view rules share one interpretation phase before they diverge into
// separate diagnostics:
//
// 1. resolve scope once
// 2. resolve each step into concrete handles or declared relationships
// 3. apply per-rule policy such as scope redundancy or request/response ordering
//
// Keeping that intermediate form explicit makes the order-sensitive response
// logic easier to read and avoids duplicating the same reference lookups in both
// dynamic-view passes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedDynamicView {
    scope: Option<ResolvedDynamicScope>,
    steps: Vec<ResolvedDynamicStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedDynamicScope {
    span: TextSpan,
    handle: SymbolHandle,
    display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedDynamicStep {
    Relationship {
        span: TextSpan,
        source: SymbolHandle,
        destination: SymbolHandle,
        source_name: String,
        destination_name: String,
        technology: Option<String>,
    },
    RelationshipReference {
        span: TextSpan,
        relationship: DeclaredRelationship,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeploymentContainmentRelation {
    SourceAncestor,
    DestinationAncestor,
}

fn deployment_semantic_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    // Deployment topology validation also works from resolved endpoint references,
    // but it stays separate from the view family because it reasons about
    // deployment containment rather than view composition. Keep the wrapper even
    // with one rule so later deployment-only checks have one obvious entry point.
    deployment_parent_child_relationship_diagnostics(documents, documents_by_id, reference_tables)
}

fn deployment_parent_child_relationship_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for relationship in &document.relationship_facts {
            let Some(source_handle) = resolved_reference_target(
                document,
                ReferenceKind::DeploymentRelationshipSource,
                relationship.source.span,
                reference_tables,
            ) else {
                continue;
            };
            let Some(destination_handle) = resolved_reference_target(
                document,
                ReferenceKind::DeploymentRelationshipDestination,
                relationship.destination.span,
                reference_tables,
            ) else {
                continue;
            };
            let Some(source_symbol) = symbol_for_handle(documents_by_id, &source_handle) else {
                continue;
            };
            let Some(destination_symbol) = symbol_for_handle(documents_by_id, &destination_handle)
            else {
                continue;
            };
            // Deployment endpoint references should already resolve to deployment
            // symbols. Keep the explicit guard so any broader future resolution
            // change fails closed instead of emitting topology diagnostics against
            // model-layer elements.
            if !is_deployment_element_kind(source_symbol.kind)
                || !is_deployment_element_kind(destination_symbol.kind)
            {
                continue;
            }
            let Some(relation) = deployment_containment_relation(
                documents_by_id,
                &source_handle,
                &destination_handle,
            ) else {
                continue;
            };

            let mut diagnostic = RuledDiagnostic::deployment_parent_child_relationship(
                &document.document_id,
                relationship.span,
            );
            let (ancestor_handle, ancestor_symbol, descendant_handle, descendant_symbol) =
                match relation {
                    DeploymentContainmentRelation::SourceAncestor => (
                        &source_handle,
                        source_symbol,
                        &destination_handle,
                        destination_symbol,
                    ),
                    DeploymentContainmentRelation::DestinationAncestor => (
                        &destination_handle,
                        destination_symbol,
                        &source_handle,
                        source_symbol,
                    ),
                };
            diagnostic.annotate(secondary_annotation(
                &document.document_id,
                ancestor_handle.document(),
                ancestor_symbol.span,
                format!(
                    "ancestor deployment element {} is declared here",
                    ancestor_symbol.display_name
                ),
            ));
            diagnostic.annotate(secondary_annotation(
                &document.document_id,
                descendant_handle.document(),
                descendant_symbol.span,
                format!(
                    "descendant deployment element {} is declared here",
                    descendant_symbol.display_name
                ),
            ));
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImageRendererRequirement {
    property_name: &'static str,
    service_name: &'static str,
}

fn resource_semantic_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    // Resource-path and image-view diagnostics form one filesystem-backed rule
    // family, so keep them together instead of interleaving them with view
    // topology checks.
    let mut diagnostics = documentation_resource_diagnostics(documents);
    diagnostics.extend(image_resource_diagnostics(documents));
    diagnostics
}

fn documentation_resource_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for directive in &document.resource_directives {
            let Some(path) = resolve_local_resource_path(&document.document_id, &directive.path)
            else {
                continue;
            };
            let Some(message) = documentation_resource_path_message(directive, &path) else {
                continue;
            };
            diagnostics.push(RuledDiagnostic::invalid_documentation_path(
                &document.document_id,
                message,
                directive.path.span,
            ));
        }
    }

    diagnostics
}

fn documentation_resource_path_message(
    directive: &ResourceDirectiveFact,
    path: &Path,
) -> Option<String> {
    match fs::metadata(path) {
        Ok(metadata) => match directive.kind {
            ResourceDirectiveKind::Docs => None,
            ResourceDirectiveKind::Adrs if metadata.is_dir() => None,
            ResourceDirectiveKind::Adrs => Some(format!(
                "Documentation path {} is not a directory",
                path.display()
            )),
        },
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Some(format!(
                "Documentation path {} does not exist",
                path.display()
            ))
        }
        Err(error) => Some(error.to_string()),
    }
}

fn image_resource_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let viewset_property_names = documents
        .iter()
        .filter(|document| !document.has_syntax_errors)
        .flat_map(|document| {
            document
                .property_facts
                .iter()
                .filter(|property| property.container_node_kind == "views_block")
                .map(|property| property.name.normalized_text.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            if view.kind != ViewKind::Image {
                continue;
            }

            for source in &view.image_sources {
                if let Some(requirement) = required_image_renderer(source.kind)
                    && !image_renderer_property_is_defined(
                        document,
                        view,
                        &viewset_property_names,
                        requirement.property_name,
                    )
                {
                    diagnostics.push(RuledDiagnostic::missing_image_renderer_property(
                        &document.document_id,
                        requirement.property_name,
                        requirement.service_name,
                        source.span,
                    ));
                }

                let Some(path) = resolve_local_resource_path(&document.document_id, &source.value)
                else {
                    continue;
                };
                let Some(message) = image_source_path_message(source.kind, &path) else {
                    continue;
                };
                diagnostics.push(RuledDiagnostic::invalid_image_source(
                    &document.document_id,
                    message,
                    source.value.span,
                ));
            }
        }
    }

    diagnostics
}

const fn required_image_renderer(kind: ImageSourceKind) -> Option<ImageRendererRequirement> {
    Some(match kind {
        ImageSourceKind::PlantUml => ImageRendererRequirement {
            property_name: "plantuml.url",
            service_name: "PlantUML",
        },
        ImageSourceKind::Mermaid => ImageRendererRequirement {
            property_name: "mermaid.url",
            service_name: "Mermaid",
        },
        ImageSourceKind::Kroki => ImageRendererRequirement {
            property_name: "kroki.url",
            service_name: "Kroki",
        },
        ImageSourceKind::Image => return None,
    })
}

fn image_renderer_property_is_defined(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    viewset_property_names: &BTreeSet<String>,
    property_name: &str,
) -> bool {
    viewset_property_names.contains(property_name)
        || image_view_local_property_is_defined(document, view, property_name)
}

fn image_view_local_property_is_defined(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    property_name: &str,
) -> bool {
    let Some(body_span) = view.body_span else {
        return false;
    };

    document.property_facts.iter().any(|property| {
        property.name.normalized_text == property_name && span_within(body_span, property.span)
    })
}

fn image_source_path_message(kind: ImageSourceKind, path: &Path) -> Option<String> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => None,
        Ok(metadata) if metadata.is_dir() => Some(match kind {
            ImageSourceKind::Image => format!("{} is not a file", path.display()),
            ImageSourceKind::PlantUml | ImageSourceKind::Mermaid | ImageSourceKind::Kroki => {
                "Is a directory".to_owned()
            }
        }),
        Ok(_) => Some(format!("{} is not a file", path.display())),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Some(format!("The file at {} does not exist", path.display()))
        }
        Err(error) => Some(error.to_string()),
    }
}

fn view_semantic_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    // All four view rules read from the same extracted `ViewFact` surface, so keep
    // them together as one post-reference pass instead of scattering view-specific
    // checks across unrelated binding code paths.
    let views_by_key = index_views_by_key(documents);
    let declared_relationships =
        collect_declared_relationships(documents, documents_by_id, reference_tables);

    let mut diagnostics = Vec::new();
    diagnostics.extend(filtered_view_autolayout_diagnostics(
        documents,
        documents_by_id,
        &views_by_key,
    ));
    diagnostics.extend(invalid_view_element_diagnostics(
        documents,
        documents_by_id,
        reference_tables,
    ));
    diagnostics.extend(dynamic_view_scope_redundancy_diagnostics(
        documents,
        documents_by_id,
        bindings,
        reference_tables,
        &declared_relationships,
    ));
    diagnostics.extend(dynamic_view_relationship_diagnostics(
        documents,
        documents_by_id,
        bindings,
        reference_tables,
        &declared_relationships,
    ));
    diagnostics
}

fn filtered_view_autolayout_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    views_by_key: &BTreeMap<String, Vec<ViewLocation>>,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            if view.kind != ViewKind::Filtered {
                continue;
            }

            let Some(base_key) = view.base_key.as_ref() else {
                continue;
            };
            let Some(base_view_locations) = views_by_key.get(&base_key.normalized_text) else {
                continue;
            };
            let [base_view_location] = base_view_locations.as_slice() else {
                continue;
            };
            let Some(base_document) = documents_by_id.get(&base_view_location.document) else {
                continue;
            };
            if base_document.has_syntax_errors {
                continue;
            }
            let Some(base_view) = base_document.view_facts.get(base_view_location.view_index) else {
                continue;
            };
            let Some(auto_layout) = base_view.auto_layout.as_ref() else {
                continue;
            };

            let mut diagnostic = RuledDiagnostic::filtered_view_autolayout_mismatch(
                &document.document_id,
                &base_key.normalized_text,
                base_key.span,
            );
            diagnostic.annotate(secondary_annotation(
                &document.document_id,
                &base_document.document_id,
                auto_layout.span,
                "base view enables automatic layout here",
            ));
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

fn invalid_view_element_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            if !matches!(
                view.kind,
                ViewKind::SystemLandscape
                    | ViewKind::SystemContext
                    | ViewKind::Container
                    | ViewKind::Component
            ) {
                continue;
            }

            push_invalid_view_value_diagnostics(
                document,
                view,
                &view.include_values,
                ReferenceKind::ViewInclude,
                documents_by_id,
                reference_tables,
                &mut diagnostics,
            );
            push_invalid_view_value_diagnostics(
                document,
                view,
                &view.animation_values,
                ReferenceKind::ViewAnimation,
                documents_by_id,
                reference_tables,
                &mut diagnostics,
            );
        }
    }

    diagnostics
}

fn push_invalid_view_value_diagnostics(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    values: &[ValueFact],
    reference_kind: ReferenceKind,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
    diagnostics: &mut Vec<RuledDiagnostic>,
) {
    for value in values {
        let Some(target_handle) =
            resolved_reference_target(document, reference_kind, value.span, reference_tables)
        else {
            continue;
        };
        let Some(target_symbol) = symbol_for_handle(documents_by_id, &target_handle) else {
            continue;
        };
        if target_symbol.kind == SymbolKind::Relationship {
            continue;
        }
        if is_view_element_kind_allowed(view.kind, target_symbol.kind) {
            continue;
        }

        diagnostics.push(RuledDiagnostic::invalid_view_element(
            &document.document_id,
            &value.normalized_text,
            value.span,
        ));
    }
}

fn dynamic_view_scope_redundancy_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            let Some(resolved_view) = resolved_dynamic_view(
                document,
                view,
                documents_by_id,
                bindings,
                reference_tables,
                declared_relationships,
            ) else {
                continue;
            };
            let Some(scope) = resolved_view.scope.as_ref() else {
                continue;
            };

            for step in &resolved_view.steps {
                match step {
                    ResolvedDynamicStep::Relationship {
                        span,
                        source,
                        destination,
                        ..
                    } => {
                        if *source != scope.handle && *destination != scope.handle {
                            continue;
                        }

                        let diagnostic = dynamic_view_scope_diagnostic(
                            &document.document_id,
                            scope,
                            *span,
                        );
                        diagnostics.push(diagnostic);
                    }
                    ResolvedDynamicStep::RelationshipReference { span, relationship } => {
                        if relationship.source != scope.handle && relationship.destination != scope.handle
                        {
                            continue;
                        }

                        let mut diagnostic = dynamic_view_scope_diagnostic(
                            &document.document_id,
                            scope,
                            *span,
                        );
                        diagnostic.annotate(dynamic_view_scope_relationship_annotation(
                            &document.document_id,
                            &scope.display_name,
                            relationship,
                        ));
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    diagnostics
}

fn dynamic_view_scope_diagnostic(
    document: &DocumentId,
    scope: &ResolvedDynamicScope,
    step_span: TextSpan,
) -> RuledDiagnostic {
    let mut diagnostic =
        RuledDiagnostic::dynamic_view_scope_redundancy(document, &scope.display_name, step_span);
    diagnostic.annotate(secondary_annotation(
        document,
        document,
        scope.span,
        "view scope is declared here",
    ));
    diagnostic
}

fn dynamic_view_scope_relationship_annotation(
    primary_document: &DocumentId,
    scope_name: &str,
    relationship: &DeclaredRelationship,
) -> Annotation {
    secondary_annotation(
        primary_document,
        &relationship.document,
        relationship.span,
        format!("referenced relationship here already includes {scope_name}"),
    )
}

fn dynamic_view_relationship_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            let Some(resolved_view) = resolved_dynamic_view(
                document,
                view,
                documents_by_id,
                bindings,
                reference_tables,
                declared_relationships,
            ) else {
                continue;
            };

            // Upstream treats a reverse-direction step as a valid response only
            // after the forward request has already appeared in the same dynamic
            // view. Keep that history explicit so both identifier-written steps and
            // named-relationship steps participate in the same ordering rule.
            let scope_handle = resolved_view.scope.as_ref().map(|scope| &scope.handle);
            let mut seen_relationships = BTreeSet::<RelationshipLocation>::new();

            for step in &resolved_view.steps {
                match step {
                    ResolvedDynamicStep::RelationshipReference { relationship, .. } => {
                        seen_relationships
                            .insert(RelationshipLocation::from_relationship(relationship));
                    }
                    ResolvedDynamicStep::Relationship {
                        span,
                        source,
                        destination,
                        source_name,
                        destination_name,
                        technology,
                    } => {
                        if scope_handle.is_some_and(|scope_handle| {
                            *scope_handle == *source || *scope_handle == *destination
                        }) {
                            continue;
                        }
                        let technology = technology.as_deref();

                        if let Some(relationship) = matching_declared_relationship(
                            source,
                            destination,
                            technology,
                            declared_relationships,
                        ) {
                            seen_relationships
                                .insert(RelationshipLocation::from_relationship(relationship));
                            continue;
                        }

                        if response_relationship_is_in_view(
                            source,
                            destination,
                            technology,
                            declared_relationships,
                            &seen_relationships,
                        ) {
                            continue;
                        }

                        let mut diagnostic = RuledDiagnostic::dynamic_view_relationship_mismatch(
                            &document.document_id,
                            source_name,
                            destination_name,
                            technology,
                            *span,
                        );
                        if let Some(annotation) = dynamic_relationship_annotation(
                            &document.document_id,
                            source,
                            destination,
                            technology,
                            declared_relationships,
                        ) {
                            diagnostic.annotate(annotation);
                        }
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    diagnostics
}

fn resolved_dynamic_view(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Option<ResolvedDynamicView> {
    if view.kind != ViewKind::Dynamic {
        return None;
    }

    let scope = view.scope.as_ref().and_then(|scope| {
        let handle =
            resolved_reference_target(document, ReferenceKind::ViewScope, scope.span, reference_tables)?;
        let symbol = symbol_for_handle(documents_by_id, &handle)?;
        Some(ResolvedDynamicScope {
            span: scope.span,
            handle,
            display_name: symbol.display_name.clone(),
        })
    });

    let steps = view
        .dynamic_steps
        .iter()
        .filter_map(|step| {
            // If a step cannot be resolved, the reference layer has already
            // emitted the appropriate unresolved/ambiguous diagnostic. The
            // view-specific passes only need the successfully resolved subset.
            resolve_dynamic_step(
                document,
                step,
                documents_by_id,
                bindings,
                reference_tables,
                declared_relationships,
            )
        })
        .collect();

    Some(ResolvedDynamicView { scope, steps })
}

fn resolve_dynamic_step(
    document: &WorkspaceSemanticDocumentFacts,
    step: &DynamicViewStepFact,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Option<ResolvedDynamicStep> {
    match step {
        DynamicViewStepFact::Relationship(step) => {
            let source =
                resolved_reference_target(document, ReferenceKind::RelationshipSource, step.source.span, reference_tables)?;
            let destination = resolved_reference_target(
                document,
                ReferenceKind::RelationshipDestination,
                step.destination.span,
                reference_tables,
            )?;
            let source_symbol = symbol_for_handle(documents_by_id, &source)?;
            let destination_symbol = symbol_for_handle(documents_by_id, &destination)?;

            Some(ResolvedDynamicStep::Relationship {
                span: step.span,
                source,
                destination,
                source_name: source_symbol.display_name.clone(),
                destination_name: destination_symbol.display_name.clone(),
                technology: step
                    .technology
                    .as_ref()
                    .map(|value| value.normalized_text.clone()),
            })
        }
        DynamicViewStepFact::RelationshipReference(step) => {
            let relationship_handle =
                resolved_relationship_binding(&step.relationship.normalized_text, bindings)?;
            let relationship =
                declared_relationship_for_handle(&relationship_handle, declared_relationships)?;

            Some(ResolvedDynamicStep::RelationshipReference {
                span: step.span,
                relationship: relationship.clone(),
            })
        }
    }
}

fn index_views_by_key(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> BTreeMap<String, Vec<ViewLocation>> {
    let mut views_by_key = BTreeMap::<String, Vec<ViewLocation>>::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for (view_index, view) in document.view_facts.iter().enumerate() {
            let Some(key) = view.key.as_ref() else {
                continue;
            };
            views_by_key
                .entry(key.normalized_text.clone())
                .or_default()
                .push(ViewLocation {
                    document: document.document_id.clone(),
                    view_index,
                });
        }
    }

    views_by_key
}

fn collect_declared_relationships(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<DeclaredRelationship> {
    let mut relationships = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        // Relationship facts already carry spans, while the symbol table owns the
        // stable symbol IDs/handles. Index the relationship symbols once per
        // document so later collection can join those two views of the same
        // declaration without rescanning the whole symbol list for every fact.
        let relationship_handles = document
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Relationship)
            .map(|symbol| {
                (
                    symbol.span,
                    SymbolHandle::new(document.document_id.clone(), symbol.id),
                )
            })
            .collect::<BTreeMap<_, _>>();

        for relationship in &document.relationship_facts {
            let Some(source) = resolved_relationship_target(
                document,
                ReferenceKind::RelationshipSource,
                ReferenceKind::DeploymentRelationshipSource,
                relationship.source.span,
                reference_tables,
            ) else {
                continue;
            };
            let Some(destination) = resolved_relationship_target(
                document,
                ReferenceKind::RelationshipDestination,
                ReferenceKind::DeploymentRelationshipDestination,
                relationship.destination.span,
                reference_tables,
            ) else {
                continue;
            };
            let Some(source_symbol) = symbol_for_handle(documents_by_id, &source) else {
                continue;
            };
            let Some(destination_symbol) = symbol_for_handle(documents_by_id, &destination) else {
                continue;
            };
            if !is_model_element_kind(source_symbol.kind)
                || !is_model_element_kind(destination_symbol.kind)
            {
                continue;
            }

            relationships.push(DeclaredRelationship {
                handle: relationship_handles.get(&relationship.span).cloned(),
                document: document.document_id.clone(),
                span: relationship.span,
                source,
                destination,
                technology: relationship
                    .technology
                    .as_ref()
                    .map(|value| value.normalized_text.clone()),
            });
        }
    }

    relationships
}

fn resolved_relationship_target(
    document: &WorkspaceSemanticDocumentFacts,
    primary_kind: ReferenceKind,
    fallback_kind: ReferenceKind,
    span: TextSpan,
    reference_tables: &WorkspaceReferenceTables,
) -> Option<SymbolHandle> {
    resolved_reference_target(document, primary_kind, span, reference_tables)
        .or_else(|| resolved_reference_target(document, fallback_kind, span, reference_tables))
}

fn resolved_reference_target(
    document: &WorkspaceSemanticDocumentFacts,
    reference_kind: ReferenceKind,
    span: TextSpan,
    reference_tables: &WorkspaceReferenceTables,
) -> Option<SymbolHandle> {
    // Semantic rules talk about source spans and reference roles, not raw
    // reference indices. Route everything through the prebuilt lookup so callers
    // do not need to know how the reference table is stored internally.
    reference_tables
        .resolved_targets
        .get(&document.document_id)?
        .get(&reference_lookup_key(reference_kind, span))
        .cloned()
}

const fn reference_lookup_key(reference_kind: ReferenceKind, span: TextSpan) -> (u8, TextSpan) {
    (reference_kind_index(reference_kind), span)
}

const fn reference_kind_index(reference_kind: ReferenceKind) -> u8 {
    match reference_kind {
        ReferenceKind::RelationshipSource => 0,
        ReferenceKind::RelationshipDestination => 1,
        ReferenceKind::InstanceTarget => 2,
        ReferenceKind::DeploymentRelationshipSource => 3,
        ReferenceKind::DeploymentRelationshipDestination => 4,
        ReferenceKind::ViewScope => 5,
        ReferenceKind::ViewInclude => 6,
        ReferenceKind::ViewExclude => 7,
        ReferenceKind::ViewAnimation => 8,
    }
}

fn resolved_relationship_binding(
    binding_name: &str,
    bindings: &WorkspaceBindingTables,
) -> Option<SymbolHandle> {
    match resolve_reference_against_binding_table(
        binding_name,
        &bindings.unique_relationships,
        &bindings.duplicate_relationships,
    ) {
        ReferenceResolutionStatus::Resolved(handle) => Some(handle),
        _ => None,
    }
}

fn symbol_for_handle<'a>(
    documents_by_id: &'a BTreeMap<DocumentId, &'a WorkspaceSemanticDocumentFacts>,
    handle: &SymbolHandle,
) -> Option<&'a Symbol> {
    documents_by_id
        .get(handle.document())
        .and_then(|document| document.symbols.get(handle.symbol_id().0))
}

const fn is_model_element_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Person
            | SymbolKind::SoftwareSystem
            | SymbolKind::Container
            | SymbolKind::Component
    )
}

const fn is_deployment_element_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::DeploymentEnvironment
            | SymbolKind::DeploymentNode
            | SymbolKind::InfrastructureNode
            | SymbolKind::ContainerInstance
            | SymbolKind::SoftwareSystemInstance
    )
}

/// Encodes the current upstream parity boundary for view families whose include
/// and animation members we validate semantically.
///
/// Keeping the matrix centralized makes later view-rule slices read in terms of
/// "which elements may this view family show?" rather than scattering that policy
/// across individual diagnostics.
const fn is_view_element_kind_allowed(view_kind: ViewKind, symbol_kind: SymbolKind) -> bool {
    match view_kind {
        ViewKind::SystemLandscape | ViewKind::SystemContext => {
            matches!(symbol_kind, SymbolKind::Person | SymbolKind::SoftwareSystem)
        }
        ViewKind::Container => matches!(
            symbol_kind,
            SymbolKind::Person | SymbolKind::SoftwareSystem | SymbolKind::Container
        ),
        ViewKind::Component => matches!(
            symbol_kind,
            SymbolKind::Person
                | SymbolKind::SoftwareSystem
                | SymbolKind::Container
                | SymbolKind::Component
        ),
        ViewKind::Filtered
        | ViewKind::Dynamic
        | ViewKind::Deployment
        | ViewKind::Custom
        | ViewKind::Image => false,
    }
}

fn matching_declared_relationship<'a>(
    source: &SymbolHandle,
    destination: &SymbolHandle,
    technology: Option<&str>,
    declared_relationships: &'a [DeclaredRelationship],
) -> Option<&'a DeclaredRelationship> {
    declared_relationships.iter().find(|relationship| {
        relationship.source == *source
            && relationship.destination == *destination
            && relationship_technology_matches(relationship.technology.as_deref(), technology)
    })
}

fn declared_relationship_for_handle<'a>(
    handle: &SymbolHandle,
    declared_relationships: &'a [DeclaredRelationship],
) -> Option<&'a DeclaredRelationship> {
    declared_relationships
        .iter()
        .find(|relationship| relationship.handle.as_ref() == Some(handle))
}

fn deployment_containment_relation(
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    source: &SymbolHandle,
    destination: &SymbolHandle,
) -> Option<DeploymentContainmentRelation> {
    if deployment_is_ancestor(documents_by_id, source, destination) {
        Some(DeploymentContainmentRelation::SourceAncestor)
    } else if deployment_is_ancestor(documents_by_id, destination, source) {
        Some(DeploymentContainmentRelation::DestinationAncestor)
    } else {
        None
    }
}

fn deployment_is_ancestor(
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    ancestor: &SymbolHandle,
    descendant: &SymbolHandle,
) -> bool {
    let mut current = deployment_parent_handle(documents_by_id, descendant);

    while let Some(parent_handle) = current {
        if &parent_handle == ancestor {
            return true;
        }
        current = deployment_parent_handle(documents_by_id, &parent_handle);
    }

    false
}

fn deployment_parent_handle(
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    handle: &SymbolHandle,
) -> Option<SymbolHandle> {
    let symbol = symbol_for_handle(documents_by_id, handle)?;
    let parent_id = symbol.parent?;
    let document = documents_by_id.get(handle.document())?;
    let parent = document.symbols.get(parent_id.0)?;
    if !is_deployment_element_kind(parent.kind) {
        return None;
    }

    Some(SymbolHandle::new(handle.document().clone(), parent.id))
}

fn response_relationship_is_in_view(
    source: &SymbolHandle,
    destination: &SymbolHandle,
    technology: Option<&str>,
    declared_relationships: &[DeclaredRelationship],
    seen_relationships: &BTreeSet<RelationshipLocation>,
) -> bool {
    declared_relationships.iter().any(|relationship| {
        relationship.source == *destination
            && relationship.destination == *source
            && relationship_technology_matches(relationship.technology.as_deref(), technology)
            && seen_relationships.contains(&RelationshipLocation::from_relationship(relationship))
    })
}

fn dynamic_relationship_annotation(
    primary_document: &DocumentId,
    source: &SymbolHandle,
    destination: &SymbolHandle,
    technology: Option<&str>,
    declared_relationships: &[DeclaredRelationship],
) -> Option<Annotation> {
    technology?;
    let candidate = declared_relationships.iter().find(|relationship| {
        relationship.source == *source && relationship.destination == *destination
    })?;
    let message = candidate.technology.as_deref().map_or_else(
        || "declared relationship here does not declare a technology".to_owned(),
        |existing| format!("declared relationship here uses technology {existing}"),
    );

    Some(secondary_annotation(
        primary_document,
        &candidate.document,
        candidate.span,
        message,
    ))
}

fn relationship_technology_matches(
    declared_technology: Option<&str>,
    expected_technology: Option<&str>,
) -> bool {
    expected_technology
        .is_none_or(|expected_technology| declared_technology == Some(expected_technology))
}

fn secondary_annotation(
    primary_document: &DocumentId,
    related_document: &DocumentId,
    span: TextSpan,
    message: impl Into<String>,
) -> Annotation {
    let annotation = if primary_document == related_document {
        Annotation::secondary(span)
    } else {
        Annotation::secondary(span).in_document(related_document)
    };
    annotation.message(message)
}

fn element_selector_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    bindings: &WorkspaceBindingTables,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for directive in &document.element_directives {
            // TODO: Path-style selectors such as `DeploymentNode://...` need a
            // richer resolver than the current binding tables.
            if !matches!(
                directive.target.value_kind,
                DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
            ) {
                continue;
            }

            match resolve_element_selector_target(document, directive, bindings) {
                SelectorResolutionStatus::Resolved => {}
                SelectorResolutionStatus::UnresolvedNoMatch => {
                    diagnostics.push(RuledDiagnostic::unresolved_element_selector(
                        &document.document_id,
                        &directive.target.normalized_text,
                        directive.target.span,
                    ));
                }
                SelectorResolutionStatus::Ambiguous => {
                    diagnostics.push(RuledDiagnostic::ambiguous_reference(
                        &document.document_id,
                        &directive.target.normalized_text,
                        directive.target.span,
                    ));
                }
            }
        }
    }

    diagnostics
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorResolutionStatus {
    Resolved,
    UnresolvedNoMatch,
    Ambiguous,
}

fn resolve_element_selector_target(
    document: &WorkspaceSemanticDocumentFacts,
    directive: &ElementDirectiveFact,
    bindings: &WorkspaceBindingTables,
) -> SelectorResolutionStatus {
    let raw_text = directive.target.normalized_text.as_str();
    let status = resolve_selector_target_raw_text(raw_text, bindings);
    if status != SelectorResolutionStatus::UnresolvedNoMatch {
        return status;
    }

    let Some(mode) = bindings.element_modes.get(&document.document_id).copied() else {
        return SelectorResolutionStatus::UnresolvedNoMatch;
    };
    let Some(containing_symbol) = enclosing_symbol_for_span(document, directive.span) else {
        return SelectorResolutionStatus::UnresolvedNoMatch;
    };

    for prefix in contextual_selector_prefixes(&document.symbols, containing_symbol, mode) {
        let candidate = format!("{prefix}.{raw_text}");
        let contextual_status = resolve_selector_target_raw_text(&candidate, bindings);
        if contextual_status != SelectorResolutionStatus::UnresolvedNoMatch {
            return contextual_status;
        }
    }

    SelectorResolutionStatus::UnresolvedNoMatch
}

fn resolve_selector_target_raw_text(
    raw_text: &str,
    bindings: &WorkspaceBindingTables,
) -> SelectorResolutionStatus {
    if bindings.duplicate_elements.contains_key(raw_text)
        || bindings.duplicate_deployments.contains_key(raw_text)
    {
        return SelectorResolutionStatus::Ambiguous;
    }

    match (
        bindings.unique_elements.get(raw_text),
        bindings.unique_deployments.get(raw_text),
    ) {
        (Some(_), Some(_)) => SelectorResolutionStatus::Ambiguous,
        (Some(_), None) | (None, Some(_)) => SelectorResolutionStatus::Resolved,
        (None, None) => SelectorResolutionStatus::UnresolvedNoMatch,
    }
}

fn enclosing_symbol_for_span(
    document: &WorkspaceSemanticDocumentFacts,
    span: TextSpan,
) -> Option<SymbolId> {
    document
        .symbols
        .iter()
        .filter(|symbol| span_within(symbol.span, span))
        .min_by_key(|symbol| symbol.span.end_byte - symbol.span.start_byte)
        .map(|symbol| symbol.id)
}

fn resolve_reference_status(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    // Syntax-role kinds remain useful to the LSP and diagnostics, but bounded
    // workspace resolution really depends on which binding family one reference
    // is allowed to target.
    let status = match reference.kind {
        ReferenceKind::RelationshipSource
        | ReferenceKind::RelationshipDestination
        | ReferenceKind::InstanceTarget
        | ReferenceKind::DeploymentRelationshipSource
        | ReferenceKind::DeploymentRelationshipDestination
        | ReferenceKind::ViewScope
        | ReferenceKind::ViewInclude
        | ReferenceKind::ViewExclude
        | ReferenceKind::ViewAnimation => {
            resolve_reference_against_target_hint(reference, bindings)
        }
    };

    if status == ReferenceResolutionStatus::UnresolvedNoMatch {
        let contextual_status =
            resolve_reference_with_symbol_context(document, reference, bindings);
        if contextual_status == ReferenceResolutionStatus::UnresolvedNoMatch {
            resolve_reference_with_selector_context(document, reference, bindings)
        } else {
            contextual_status
        }
    } else {
        status
    }
}

fn resolve_reference_with_symbol_context(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    let Some(containing_symbol) = reference.containing_symbol else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };
    let Some(mode) = bindings.element_modes.get(&document.document_id).copied() else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };

    for prefix in contextual_reference_prefixes(
        &document.symbols,
        containing_symbol,
        mode,
        reference.target_hint,
    ) {
        let contextual_raw_text = format!("{prefix}.{}", reference.raw_text);
        let status = match reference.target_hint {
            ReferenceTargetHint::Element => resolve_reference_against_element_table(
                &contextual_raw_text,
                &bindings.unique_elements,
                &bindings.duplicate_elements,
            ),
            ReferenceTargetHint::Deployment => resolve_reference_against_binding_table(
                &contextual_raw_text,
                &bindings.unique_deployments,
                &bindings.duplicate_deployments,
            ),
            ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => {
                ReferenceResolutionStatus::UnresolvedNoMatch
            }
        };
        if status != ReferenceResolutionStatus::UnresolvedNoMatch {
            return status;
        }
    }

    ReferenceResolutionStatus::UnresolvedNoMatch
}

fn contextual_reference_prefixes(
    symbols: &[Symbol],
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
    target_hint: ReferenceTargetHint,
) -> Vec<String> {
    match target_hint {
        ReferenceTargetHint::Element => contextual_prefixes(
            symbols,
            containing_symbol,
            mode,
            &[CanonicalBindingKind::Element],
        ),
        ReferenceTargetHint::Deployment => contextual_prefixes(
            symbols,
            containing_symbol,
            mode,
            &[CanonicalBindingKind::Deployment],
        ),
        ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => {
            Vec::new()
        }
    }
}

fn contextual_selector_prefixes(
    symbols: &[Symbol],
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
) -> Vec<String> {
    contextual_prefixes(
        symbols,
        containing_symbol,
        mode,
        &[
            CanonicalBindingKind::Element,
            CanonicalBindingKind::Deployment,
        ],
    )
}

fn contextual_prefixes(
    symbols: &[Symbol],
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
    binding_kinds: &[CanonicalBindingKind],
) -> Vec<String> {
    // Symbol-context references and `!element` selectors both walk outward
    // through the same ancestor chain. The only difference is whether one pass
    // should consider element bindings, deployment bindings, or both.
    let mut prefixes = Vec::new();
    let mut current = Some(containing_symbol);

    while let Some(symbol_id) = current {
        let symbol = symbols
            .get(symbol_id.0)
            .expect("BUG: contextual prefix symbol should exist");
        for binding_kind in binding_kinds {
            if let Some(prefix) = canonical_binding_key(symbols, symbol_id, mode, *binding_kind) {
                prefixes.push(prefix);
            }
        }
        current = symbol.parent;
    }

    prefixes
}

fn resolve_reference_with_selector_context(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    let Some(selector_target) = enclosing_element_selector_target(document, reference.span) else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };
    if !matches!(
        selector_target.value_kind,
        DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
    ) {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    }

    let contextual_raw_text = format!("{}.{}", selector_target.normalized_text, reference.raw_text);
    match reference.target_hint {
        ReferenceTargetHint::Element => resolve_reference_against_element_table(
            &contextual_raw_text,
            &bindings.unique_elements,
            &bindings.duplicate_elements,
        ),
        ReferenceTargetHint::Deployment => resolve_reference_against_binding_table(
            &contextual_raw_text,
            &bindings.unique_deployments,
            &bindings.duplicate_deployments,
        ),
        ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => {
            ReferenceResolutionStatus::UnresolvedNoMatch
        }
    }
}

fn enclosing_element_selector_target(
    document: &WorkspaceSemanticDocumentFacts,
    span: TextSpan,
) -> Option<&crate::ValueFact> {
    document
        .element_directives
        .iter()
        .filter(|directive| span_within(directive.span, span))
        .min_by_key(|directive| directive.span.end_byte - directive.span.start_byte)
        .map(|directive| &directive.target)
}

const fn span_within(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start_byte <= inner.start_byte && inner.end_byte <= outer.end_byte
}

fn resolve_reference_against_target_hint(
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    match reference.target_hint {
        ReferenceTargetHint::Element => resolve_reference_against_element_table(
            &reference.raw_text,
            &bindings.unique_elements,
            &bindings.duplicate_elements,
        ),
        ReferenceTargetHint::Deployment => resolve_reference_against_binding_table(
            &reference.raw_text,
            &bindings.unique_deployments,
            &bindings.duplicate_deployments,
        ),
        ReferenceTargetHint::Relationship => resolve_reference_against_binding_table(
            &reference.raw_text,
            &bindings.unique_relationships,
            &bindings.duplicate_relationships,
        ),
        ReferenceTargetHint::ElementOrRelationship => resolve_view_include_reference(
            &reference.raw_text,
            &bindings.unique_elements,
            &bindings.duplicate_elements,
            &bindings.unique_relationships,
            &bindings.duplicate_relationships,
        ),
    }
}

fn resolve_reference_against_element_table(
    raw_text: &str,
    unique_element_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_element_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
) -> ReferenceResolutionStatus {
    // Keep the element-flavoured wrapper even though it currently delegates
    // directly so the call sites still read in terms of binding families rather
    // than raw table plumbing.
    resolve_reference_against_binding_table(
        raw_text,
        unique_element_bindings,
        duplicate_element_bindings,
    )
}

fn resolve_reference_against_binding_table(
    raw_text: &str,
    unique_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
) -> ReferenceResolutionStatus {
    if duplicate_bindings.contains_key(raw_text) {
        return ReferenceResolutionStatus::AmbiguousDuplicateBinding;
    }

    unique_bindings.get(raw_text).cloned().map_or(
        ReferenceResolutionStatus::UnresolvedNoMatch,
        ReferenceResolutionStatus::Resolved,
    )
}

fn resolve_view_include_reference(
    raw_text: &str,
    unique_element_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_element_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
    unique_relationship_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_relationship_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
) -> ReferenceResolutionStatus {
    if duplicate_element_bindings.contains_key(raw_text)
        || duplicate_relationship_bindings.contains_key(raw_text)
    {
        return ReferenceResolutionStatus::AmbiguousDuplicateBinding;
    }

    match (
        unique_element_bindings.get(raw_text),
        unique_relationship_bindings.get(raw_text),
    ) {
        (Some(_), Some(_)) => ReferenceResolutionStatus::AmbiguousElementVsRelationship,
        (Some(symbol), None) | (None, Some(symbol)) => {
            ReferenceResolutionStatus::Resolved(symbol.clone())
        }
        (None, None) => ReferenceResolutionStatus::UnresolvedNoMatch,
    }
}

fn effective_element_identifier_mode(
    document: &WorkspaceSemanticDocumentFacts,
    inherited_workspace_mode: Option<&IdentifierMode>,
) -> ElementIdentifierMode {
    effective_element_identifier_mode_from_facts(
        &document.identifier_modes,
        inherited_workspace_mode,
    )
}

/// Derives the bounded element-identifier mode from raw directive facts.
///
/// Both workspace indexes and snapshot-only LSP helpers rely on this
/// precedence, so keeping it shared prevents drift between read-only features
/// and edit planning.
pub fn effective_element_identifier_mode_from_facts(
    identifier_modes: &[IdentifierModeFact],
    inherited_workspace_mode: Option<&IdentifierMode>,
) -> ElementIdentifierMode {
    match document_model_identifier_mode(identifier_modes)
        .or_else(|| document_workspace_identifier_mode(identifier_modes))
        .or_else(|| inherited_workspace_mode.cloned())
    {
        Some(IdentifierMode::Hierarchical) => ElementIdentifierMode::Hierarchical,
        Some(IdentifierMode::Flat) | None => ElementIdentifierMode::Flat,
        Some(IdentifierMode::Other(_)) => ElementIdentifierMode::Deferred,
    }
}

fn document_model_identifier_mode(
    identifier_modes: &[IdentifierModeFact],
) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(identifier_modes, &DirectiveContainer::Model)
}

fn document_workspace_identifier_mode(
    identifier_modes: &[IdentifierModeFact],
) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(identifier_modes, &DirectiveContainer::Workspace)
}

fn last_identifier_mode_for_container(
    identifier_modes: &[IdentifierModeFact],
    container: &DirectiveContainer,
) -> Option<IdentifierMode> {
    identifier_modes
        .iter()
        .rev()
        .find(|fact| fact.container == *container)
        .map(|fact| fact.mode.clone())
}

#[derive(Clone, Copy)]
enum CanonicalBindingKind {
    Element,
    Deployment,
}

impl CanonicalBindingKind {
    const fn allows_ancestor(self, kind: SymbolKind) -> bool {
        match self {
            Self::Element => matches!(
                kind,
                SymbolKind::Person
                    | SymbolKind::SoftwareSystem
                    | SymbolKind::Container
                    | SymbolKind::Component
            ),
            Self::Deployment => matches!(
                kind,
                SymbolKind::DeploymentEnvironment
                    | SymbolKind::DeploymentNode
                    | SymbolKind::InfrastructureNode
                    | SymbolKind::ContainerInstance
                    | SymbolKind::SoftwareSystemInstance
            ),
        }
    }
}

fn canonical_binding_key(
    symbols: &[Symbol],
    symbol_id: SymbolId,
    mode: ElementIdentifierMode,
    binding_kind: CanonicalBindingKind,
) -> Option<String> {
    let symbol = symbols.get(symbol_id.0)?;
    let binding_name = symbol.binding_name.as_deref()?;

    match mode {
        ElementIdentifierMode::Flat => Some(binding_name.to_owned()),
        ElementIdentifierMode::Deferred => None,
        ElementIdentifierMode::Hierarchical => {
            let mut segments = vec![binding_name.to_owned()];
            let mut parent = symbol.parent;

            while let Some(parent_id) = parent {
                let ancestor = symbols.get(parent_id.0)?;
                if !binding_kind.allows_ancestor(ancestor.kind) {
                    // Once the ancestor chain stops describing one canonical
                    // element/deployment path, drop the whole hierarchical key
                    // instead of emitting a truncated binding that would collide
                    // with a different legitimate declaration.
                    return None;
                }

                let ancestor_binding = ancestor.binding_name.as_deref()?;
                segments.push(ancestor_binding.to_owned());
                parent = ancestor.parent;
            }

            segments.reverse();
            Some(segments.join("."))
        }
    }
}

fn merge_semantic_diagnostics(
    workspace_indexes: &[WorkspaceIndex],
    document_instances: &BTreeMap<DocumentId, Vec<WorkspaceInstanceId>>,
) -> Vec<RuledDiagnostic> {
    // A document can participate in multiple candidate workspace instances.
    // Only publish a merged semantic diagnostic when every instance that contains
    // the document agrees on that diagnostic; otherwise editor surfaces would
    // oscillate based on whichever instance happened to be consulted first.
    let mut diagnostic_counts = BTreeMap::<DocumentId, BTreeMap<RuledDiagnostic, usize>>::new();

    for workspace_index in workspace_indexes {
        let mut per_document = BTreeMap::<DocumentId, BTreeSet<RuledDiagnostic>>::new();

        for diagnostic in workspace_index.semantic_diagnostics() {
            per_document
                .entry(
                    diagnostic
                        .document()
                        .expect("semantic diagnostics should carry documents")
                        .clone(),
                )
                .or_default()
                .insert(diagnostic.clone());
        }

        for (document, diagnostics) in per_document {
            let counts = diagnostic_counts.entry(document).or_default();
            for diagnostic in diagnostics {
                *counts.entry(diagnostic).or_default() += 1;
            }
        }
    }

    let mut merged = Vec::new();
    for (document, instances) in document_instances {
        let Some(counts) = diagnostic_counts.get(document) else {
            continue;
        };

        for (diagnostic, count) in counts {
            if *count == instances.len() {
                merged.push(diagnostic.clone());
            }
        }
    }

    sort_semantic_diagnostics(&mut merged);
    merged
}

fn sort_semantic_diagnostics(diagnostics: &mut [RuledDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.document()
            .cmp(&right.document())
            .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
            .then_with(|| left.rule.cmp(&right.rule))
            .then_with(|| left.message().cmp(right.message()))
    });
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        ptr,
        sync::Arc,
    };

    use indoc::indoc;
    use tempfile::TempDir;

    use super::{
        SymbolKind, ViewKind, WorkspaceFacts, WorkspaceIndex, WorkspaceLoader,
        document_id_from_path, is_view_element_kind_allowed,
    };

    #[test]
    fn loader_reuses_cached_document_snapshots_across_identical_loads() {
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

        let first_workspace = first
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        let second_workspace = second
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        assert!(ptr::eq(
            first_workspace.snapshot(),
            second_workspace.snapshot()
        ));

        let first_model = first
            .document(&document_id_from_path(&fixture.model_path()))
            .expect("included model document should exist");
        let second_model = second
            .document(&document_id_from_path(&fixture.model_path()))
            .expect("included model document should exist");
        assert!(ptr::eq(first_model.snapshot(), second_model.snapshot()));
    }

    #[test]
    fn view_element_matrix_matches_current_upstream_parity() {
        assert!(is_view_element_kind_allowed(
            ViewKind::SystemLandscape,
            SymbolKind::SoftwareSystem
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::SystemLandscape,
            SymbolKind::Container
        ));

        assert!(is_view_element_kind_allowed(
            ViewKind::SystemContext,
            SymbolKind::Person
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::SystemContext,
            SymbolKind::Container
        ));

        assert!(is_view_element_kind_allowed(
            ViewKind::Container,
            SymbolKind::Container
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::Container,
            SymbolKind::Component
        ));

        assert!(is_view_element_kind_allowed(
            ViewKind::Component,
            SymbolKind::Component
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::Component,
            SymbolKind::Relationship
        ));
    }

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
    fn loader_refreshes_cached_snapshot_when_override_changes() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                model {
                    user = person "User"
                }
            }
        "#});

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("first load should succeed");

        loader.set_document_override(
            fixture.workspace_path().clone(),
            indoc! {r#"
                workspace {
                    model {
                        admin = person "Admin"
                    }
                }
            "#}
            .to_owned(),
        );

        let second = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("override-backed load should succeed");

        let first_document = first
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        let second_document = second
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");

        assert!(!ptr::eq(
            first_document.snapshot(),
            second_document.snapshot()
        ));
        assert_eq!(
            second_document
                .snapshot()
                .symbols()
                .iter()
                .filter_map(|symbol| symbol.binding_name.as_deref())
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
    }

    #[test]
    fn clearing_document_overrides_restores_disk_backed_snapshot() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                model {
                    user = person "User"
                }
            }
        "#});

        let mut loader = WorkspaceLoader::new();
        loader.set_document_override(
            fixture.workspace_path().clone(),
            indoc! {r#"
                workspace {
                    model {
                        admin = person "Admin"
                    }
                }
            "#}
            .to_owned(),
        );
        let override_backed = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("override-backed load should succeed");

        loader.clear_document_overrides();
        let disk_backed = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("disk-backed reload should succeed");

        let override_document = override_backed
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        let disk_document = disk_backed
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");

        assert_eq!(
            override_document
                .snapshot()
                .symbols()
                .iter()
                .filter_map(|symbol| symbol.binding_name.as_deref())
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
        assert_eq!(
            disk_document
                .snapshot()
                .symbols()
                .iter()
                .filter_map(|symbol| symbol.binding_name.as_deref())
                .collect::<Vec<_>>(),
            vec!["user"]
        );
    }

    #[test]
    fn cached_parent_context_refreshes_when_included_child_changes() {
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
        let first_index = first
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            first_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["user"]
        );

        fixture.write_model(indoc! {r#"
            model {
                admin = person "Admin"
            }
        "#});

        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");
        let second_index = second
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            second_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
    }

    #[test]
    fn cached_parent_context_refreshes_when_grandchild_changes() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                !include "model.dsl"
            }
        "#});
        fixture.write_model(indoc! {r#"
            model {
                !include "people.dsl"
            }
        "#});
        fixture.write_file(
            "people.dsl",
            indoc! {r#"
                model {
                    user = person "User"
                }
            "#},
        );

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.root()])
            .expect("first load should succeed");
        let first_index = first
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            first_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["user"]
        );

        fixture.write_file(
            "people.dsl",
            indoc! {r#"
                model {
                    admin = person "Admin"
                }
            "#},
        );

        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");
        let second_index = second
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            second_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
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

    fn workspace_index_for_root<'a>(
        facts: &'a WorkspaceFacts,
        root_path: &Path,
    ) -> &'a WorkspaceIndex {
        let root_document = document_id_from_path(root_path);
        facts
            .workspace_indexes()
            .iter()
            .find(|index| index.root_document() == &root_document)
            .expect("workspace index for root should exist")
    }

    struct TemporaryWorkspace {
        _root_dir: TempDir,
        root: PathBuf,
        workspace_path: PathBuf,
    }

    impl TemporaryWorkspace {
        fn new(workspace_source: &str) -> Self {
            let root_dir = tempfile::tempdir().expect("tempdir should create");
            let root = root_dir
                .path()
                .canonicalize()
                .expect("tempdir path should canonicalize");
            let workspace_path = root.join("workspace.dsl");
            fs::write(&workspace_path, workspace_source).expect("workspace source should write");

            Self {
                _root_dir: root_dir,
                root,
                workspace_path,
            }
        }

        fn write_model(&self, source: &str) {
            fs::write(self.model_path(), source).expect("model source should write");
        }

        fn write_file(&self, relative_path: &str, source: &str) {
            fs::write(self.root.join(relative_path), source).expect("workspace file should write");
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn workspace_path(&self) -> &PathBuf {
            &self.workspace_path
        }

        fn model_path(&self) -> PathBuf {
            self.root.join("model.dsl")
        }
    }
}
