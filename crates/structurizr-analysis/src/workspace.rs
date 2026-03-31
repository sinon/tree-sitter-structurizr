//! Workspace discovery, include-following, and file-level include diagnostics.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use ignore::WalkBuilder;

use crate::{
    ConstantDefinition, DocumentAnalyzer, DocumentId, DocumentInput, IdentifierMode,
    IncludeDiagnostic, IncludeDirective, Reference, ReferenceKind, ReferenceTargetHint,
    SemanticDiagnostic, SymbolId, SymbolKind, TextSpan,
    includes::{DirectiveContainer, normalized_directive_value},
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
    kind: WorkspaceDocumentKind,
    discovered_by_scan: bool,
}

impl WorkspaceDocument {
    const fn new(
        snapshot: Arc<crate::DocumentSnapshot>,
        kind: WorkspaceDocumentKind,
        discovered_by_scan: bool,
    ) -> Self {
        Self {
            snapshot,
            kind,
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

/// Derived semantic index for one workspace instance.
#[derive(Debug)]
pub struct WorkspaceIndex {
    id: WorkspaceInstanceId,
    root_document: DocumentId,
    documents: Vec<DocumentId>,
    unique_element_bindings: BTreeMap<String, SymbolHandle>,
    duplicate_element_bindings: BTreeMap<String, Vec<SymbolHandle>>,
    unique_deployment_bindings: BTreeMap<String, SymbolHandle>,
    duplicate_deployment_bindings: BTreeMap<String, Vec<SymbolHandle>>,
    unique_relationship_bindings: BTreeMap<String, SymbolHandle>,
    duplicate_relationship_bindings: BTreeMap<String, Vec<SymbolHandle>>,
    reference_resolutions: BTreeMap<ReferenceHandle, ReferenceResolutionStatus>,
    references_by_target: BTreeMap<SymbolHandle, Vec<ReferenceHandle>>,
    semantic_diagnostics: Vec<SemanticDiagnostic>,
}

impl WorkspaceIndex {
    /// Returns this index's stable instance identity.
    #[must_use]
    pub const fn id(&self) -> WorkspaceInstanceId {
        self.id
    }

    /// Returns the root document that defines this workspace instance.
    #[must_use]
    pub const fn root_document(&self) -> &DocumentId {
        &self.root_document
    }

    /// Returns the discovered documents that participate in this instance.
    #[must_use]
    pub fn documents(&self) -> &[DocumentId] {
        &self.documents
    }

    /// Returns the unique element-binding table keyed by canonical binding key.
    #[must_use]
    pub const fn unique_element_bindings(&self) -> &BTreeMap<String, SymbolHandle> {
        &self.unique_element_bindings
    }

    /// Returns the duplicate element-binding sets keyed by canonical binding key.
    #[must_use]
    pub const fn duplicate_element_bindings(&self) -> &BTreeMap<String, Vec<SymbolHandle>> {
        &self.duplicate_element_bindings
    }

    /// Returns the unique deployment-binding table keyed by binding identifier.
    #[must_use]
    pub const fn unique_deployment_bindings(&self) -> &BTreeMap<String, SymbolHandle> {
        &self.unique_deployment_bindings
    }

    /// Returns the duplicate deployment-binding sets keyed by binding identifier.
    #[must_use]
    pub const fn duplicate_deployment_bindings(&self) -> &BTreeMap<String, Vec<SymbolHandle>> {
        &self.duplicate_deployment_bindings
    }

    /// Returns the unique relationship-binding table keyed by canonical binding key.
    #[must_use]
    pub const fn unique_relationship_bindings(&self) -> &BTreeMap<String, SymbolHandle> {
        &self.unique_relationship_bindings
    }

    /// Returns the duplicate relationship-binding sets keyed by canonical key.
    #[must_use]
    pub const fn duplicate_relationship_bindings(&self) -> &BTreeMap<String, Vec<SymbolHandle>> {
        &self.duplicate_relationship_bindings
    }

    /// Returns the resolution status recorded for one reference handle.
    #[must_use]
    pub fn reference_resolution(
        &self,
        handle: &ReferenceHandle,
    ) -> Option<&ReferenceResolutionStatus> {
        self.reference_resolutions.get(handle)
    }

    /// Returns every resolved reference that points at one symbol.
    pub fn references_for_symbol(
        &self,
        handle: &SymbolHandle,
    ) -> impl Iterator<Item = &ReferenceHandle> + '_ {
        self.references_by_target.get(handle).into_iter().flatten()
    }

    /// Returns the semantic diagnostics derived for this workspace instance.
    #[must_use]
    pub fn semantic_diagnostics(&self) -> &[SemanticDiagnostic] {
        &self.semantic_diagnostics
    }

    /// Returns whether the workspace instance includes one document.
    #[must_use]
    pub fn contains_document(&self, document: &DocumentId) -> bool {
        self.documents.contains(document)
    }
}

/// Multi-file discovery facts gathered from one or more workspace roots.
#[derive(Debug, Default)]
pub struct WorkspaceFacts {
    documents: Vec<WorkspaceDocument>,
    resolved_includes: Vec<ResolvedInclude>,
    include_diagnostics: Vec<IncludeDiagnostic>,
    workspace_indexes: Vec<WorkspaceIndex>,
    document_instances: BTreeMap<DocumentId, Vec<WorkspaceInstanceId>>,
    semantic_diagnostics: Vec<SemanticDiagnostic>,
}

impl WorkspaceFacts {
    /// Returns every discovered document in deterministic path order.
    #[must_use]
    pub fn documents(&self) -> &[WorkspaceDocument] {
        &self.documents
    }

    /// Returns the discovered include-following results in deterministic order.
    #[must_use]
    pub fn includes(&self) -> &[ResolvedInclude] {
        &self.resolved_includes
    }

    /// Returns include-resolution diagnostics in deterministic order.
    #[must_use]
    pub fn include_diagnostics(&self) -> &[IncludeDiagnostic] {
        &self.include_diagnostics
    }

    /// Returns include-resolution diagnostics for one document.
    pub fn include_diagnostics_for(
        &self,
        id: &DocumentId,
    ) -> impl Iterator<Item = &IncludeDiagnostic> + '_ {
        let id = id.clone();
        self.include_diagnostics
            .iter()
            .filter(move |diagnostic| diagnostic.document == id)
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
        self.document_instances.get(id).into_iter().flatten()
    }

    /// Returns every merged semantic diagnostic in deterministic order.
    #[must_use]
    pub fn semantic_diagnostics(&self) -> &[SemanticDiagnostic] {
        &self.semantic_diagnostics
    }

    /// Returns merged semantic diagnostics for one document.
    pub fn semantic_diagnostics_for(
        &self,
        id: &DocumentId,
    ) -> impl Iterator<Item = &SemanticDiagnostic> + '_ {
        let id = id.clone();
        self.semantic_diagnostics
            .iter()
            .filter(move |diagnostic| diagnostic.document == id)
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
    next_document_generation: u64,
    next_context_revision: u64,
}

#[derive(Debug)]
struct CachedWorkspaceDocument {
    snapshot: Arc<crate::DocumentSnapshot>,
    kind: WorkspaceDocumentKind,
    generation: u64,
}

#[derive(Debug, Clone)]
struct CachedProcessedDocumentContext {
    processed: ProcessedDocumentContext,
    document_generation: u64,
    child_context_revisions: Vec<u64>,
    include_validations: Vec<CachedIncludeValidation>,
    revision: u64,
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
    fn new(snapshot: crate::DocumentSnapshot, generation: u64) -> Self {
        let kind = if snapshot.is_workspace_entry() {
            WorkspaceDocumentKind::Entry
        } else {
            WorkspaceDocumentKind::Fragment
        };

        Self {
            snapshot: Arc::new(snapshot),
            kind,
            generation,
        }
    }

    fn workspace_document(&self, discovered_by_scan: bool) -> WorkspaceDocument {
        WorkspaceDocument::new(Arc::clone(&self.snapshot), self.kind, discovered_by_scan)
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
                DocumentInput::new(document_id_from_path(path), source).with_location(path.to_path_buf()),
            );
            let generation = self.next_document_generation();
            self.document_cache.insert(
                path.to_path_buf(),
                CachedWorkspaceDocument::new(snapshot, generation),
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
        self.document_cache.get(path).map(|cached| cached.generation)
    }

    fn processed_context_revision(&self, key: &DocumentContextKey) -> Option<u64> {
        self.processed_context_cache.get(key).map(|cached| cached.revision)
    }

    fn cached_processed_context(&self, key: &DocumentContextKey) -> Option<CachedProcessedDocumentContext> {
        self.processed_context_cache.get(key).cloned()
    }

    fn store_processed_context(&mut self, context: &DocumentContext, processed: ProcessedDocumentContext) {
        let child_context_revisions = processed
            .included_contexts
            .iter()
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

        self.processed_context_cache.insert(context.key.clone(), cached);
    }

    fn include_validation(&self, include: &ResolvedInclude) -> CachedIncludeValidation {
        match include.target() {
            WorkspaceIncludeTarget::RemoteUrl { url } => CachedIncludeValidation::RemoteUrl {
                url: url.clone(),
            },
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
            WorkspaceIncludeTarget::LocalDirectory { path } => CachedIncludeValidation::LocalDirectory {
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
            },
        }
    }

    const fn next_document_generation(&mut self) -> u64 {
        self.next_document_generation = self
            .next_document_generation
            .checked_add(1)
            .expect("document generation counter should not overflow");
        self.next_document_generation
    }

    const fn next_context_revision(&mut self) -> u64 {
        self.next_context_revision = self
            .next_context_revision
            .checked_add(1)
            .expect("context revision counter should not overflow");
        self.next_context_revision
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
        let mut resolved_includes = self
            .processed_contexts
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
        let (workspace_indexes, document_instances, semantic_diagnostics) =
            build_workspace_indexes(&self.loaded_documents, start_contexts, &self.processed_contexts);

        WorkspaceFacts {
            documents: self.loaded_documents.into_values().collect(),
            resolved_includes,
            include_diagnostics,
            workspace_indexes,
            document_instances,
            semantic_diagnostics,
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

    fn process_document_context(&mut self, context: DocumentContext) -> io::Result<ConstantEnvironment> {
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

        let (document_id, constant_definitions, include_directives) = {
            let document = self
                .loaded_documents
                .get(&context.path)
                .expect("BUG: document context should be loaded before processing");

            (
                document.id().clone(),
                document.snapshot().constant_definitions().to_vec(),
                document.snapshot().include_directives().to_vec(),
            )
        };

        self.active_stack.push(context.path.clone());
        let processed = (|| -> io::Result<ProcessedDocumentContext> {
            let mut current_constants = context.inherited_constants.clone();
            let mut direct_includes = Vec::new();
            let mut included_contexts = Vec::new();

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
                direct_includes,
                included_contexts,
            })
        })();
        let popped_path = self.active_stack.pop();
        debug_assert_eq!(popped_path.as_deref(), Some(context.path.as_path()));

        let processed = processed?;
        let exported_constants = processed.exported_constants.clone();
        self.session.store_processed_context(&context, processed.clone());
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

            let Some(current_generation) = self.session.document_generation(&context_key.path) else {
                return Ok(false);
            };
            if current_generation != cached.document_generation {
                return Ok(false);
            }

            for (child_context, expected_revision) in cached
                .processed
                .included_contexts
                .iter()
                .zip(&cached.child_context_revisions)
            {
                let Some(current_revision) = self.session.processed_context_revision(child_context) else {
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

            for child_context in &cached.processed.included_contexts {
                self.materialize_cached_context_tree(child_context, visiting)?;
            }

            self.processed_contexts
                .insert(context_key.clone(), cached.processed);
            Ok(())
        })();
        let _ = visiting.remove(context_key);
        materialized
    }

    fn include_validation_is_fresh(&mut self, validation: &CachedIncludeValidation) -> io::Result<bool> {
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
                let allowed_root = path.parent().expect("directory include path should have a parent");
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
                    if self.session.document_generation(current_path) != Some(*expected_generation) {
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

fn include_diagnostics(resolved_includes: &[ResolvedInclude]) -> Vec<IncludeDiagnostic> {
    let cycle_include_indices = cycle_include_indices(resolved_includes);
    let mut diagnostics = Vec::new();

    for (index, include) in resolved_includes.iter().enumerate() {
        match include.target() {
            WorkspaceIncludeTarget::MissingLocalPath { .. } => {
                diagnostics.push(IncludeDiagnostic::missing_local_target(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::UnsupportedLocalPath { .. } => {
                diagnostics.push(IncludeDiagnostic::escapes_allowed_subtree(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::RemoteUrl { .. } => {
                diagnostics.push(IncludeDiagnostic::unsupported_remote_target(
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
            diagnostics.push(IncludeDiagnostic::include_cycle(
                include.including_document(),
                include.target_text(),
                include.span(),
                include.value_span(),
            ));
        }
    }

    diagnostics.sort_by(|left, right| {
        left.document
            .cmp(&right.document)
            .then_with(|| left.span.start_byte.cmp(&right.span.start_byte))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.target_text.cmp(&right.target_text))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementIdentifierMode {
    Flat,
    Hierarchical,
    Deferred,
}

fn build_workspace_indexes(
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
    start_contexts: &[DocumentContext],
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
) -> (
    Vec<WorkspaceIndex>,
    BTreeMap<DocumentId, Vec<WorkspaceInstanceId>>,
    Vec<SemanticDiagnostic>,
) {
    let documents_by_id = loaded_documents
        .values()
        .map(|document| (document.id().clone(), document))
        .collect::<BTreeMap<_, _>>();

    let mut workspace_indexes = Vec::new();
    let mut document_instances = BTreeMap::<DocumentId, Vec<WorkspaceInstanceId>>::new();

    for (ordinal, start_context) in start_contexts.iter().enumerate() {
        let instance_id = WorkspaceInstanceId(ordinal);
        let mut visited_contexts = BTreeSet::new();
        let mut seen_documents = BTreeSet::new();
        let mut instance_documents = Vec::new();
        collect_instance_documents(
            &start_context.key,
            processed_contexts,
            &mut visited_contexts,
            &mut seen_documents,
            &mut instance_documents,
        );

        let root_document = document_id_from_path(&start_context.path);
        let index = build_workspace_index(
            instance_id,
            root_document,
            &instance_documents,
            &documents_by_id,
        );

        for document in index.documents() {
            document_instances
                .entry(document.clone())
                .or_default()
                .push(index.id());
        }

        workspace_indexes.push(index);
    }

    let semantic_diagnostics = merge_semantic_diagnostics(&workspace_indexes, &document_instances);

    (workspace_indexes, document_instances, semantic_diagnostics)
}

fn collect_instance_documents(
    context_key: &DocumentContextKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    visited_contexts: &mut BTreeSet<DocumentContextKey>,
    seen_documents: &mut BTreeSet<DocumentId>,
    instance_documents: &mut Vec<DocumentId>,
) {
    if !visited_contexts.insert(context_key.clone()) {
        return;
    }

    let document_id = document_id_from_path(&context_key.path);
    if seen_documents.insert(document_id.clone()) {
        instance_documents.push(document_id);
    }

    let Some(processed_context) = processed_contexts.get(context_key) else {
        return;
    };

    for child_context in &processed_context.included_contexts {
        collect_instance_documents(
            child_context,
            processed_contexts,
            visited_contexts,
            seen_documents,
            instance_documents,
        );
    }
}

fn build_workspace_index(
    id: WorkspaceInstanceId,
    root_document: DocumentId,
    documents: &[DocumentId],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
) -> WorkspaceIndex {
    let root_snapshot = documents_by_id
        .get(&root_document)
        .expect("BUG: workspace-index root document should exist")
        .snapshot();
    let inherited_workspace_mode = document_workspace_identifier_mode(root_snapshot);
    let bindings = build_binding_tables(
        documents,
        documents_by_id,
        inherited_workspace_mode.as_ref(),
    );
    let mut semantic_diagnostics = bindings.semantic_diagnostics.clone();
    let reference_tables = build_reference_resolution_tables(documents, documents_by_id, &bindings);
    semantic_diagnostics.extend(reference_tables.semantic_diagnostics);

    let mut references_by_target = reference_tables.references_by_target;
    for references in references_by_target.values_mut() {
        references.sort();
        references.dedup();
    }
    sort_semantic_diagnostics(&mut semantic_diagnostics);

    WorkspaceIndex {
        id,
        root_document,
        documents: documents.to_vec(),
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

struct WorkspaceBindingTables {
    unique_elements: BTreeMap<String, SymbolHandle>,
    duplicate_elements: BTreeMap<String, Vec<SymbolHandle>>,
    unique_deployments: BTreeMap<String, SymbolHandle>,
    duplicate_deployments: BTreeMap<String, Vec<SymbolHandle>>,
    unique_relationships: BTreeMap<String, SymbolHandle>,
    duplicate_relationships: BTreeMap<String, Vec<SymbolHandle>>,
    semantic_diagnostics: Vec<SemanticDiagnostic>,
}

fn build_binding_tables(
    documents: &[DocumentId],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
    inherited_workspace_mode: Option<&IdentifierMode>,
) -> WorkspaceBindingTables {
    let mut element_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();
    let mut deployment_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();
    let mut relationship_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();

    for document_id in documents {
        let document = documents_by_id
            .get(document_id)
            .expect("BUG: workspace-index document should exist");
        let snapshot = document.snapshot();
        let element_mode = effective_element_identifier_mode(snapshot, inherited_workspace_mode);

        for symbol in snapshot.symbols() {
            let Some(binding_name) = symbol.binding_name.as_deref() else {
                continue;
            };

            let handle = SymbolHandle {
                document: document_id.clone(),
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
                    let Some(binding_key) =
                        canonical_element_binding_key(snapshot, symbol.id, element_mode)
                    else {
                        continue;
                    };

                    element_bindings
                        .entry(binding_key)
                        .or_default()
                        .push(handle);
                }
                SymbolKind::DeploymentNode
                | SymbolKind::InfrastructureNode
                | SymbolKind::ContainerInstance
                | SymbolKind::SoftwareSystemInstance => {
                    deployment_bindings
                        .entry(binding_name.to_owned())
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
        documents_by_id,
        &mut semantic_diagnostics,
    );
    push_duplicate_binding_diagnostics(
        "deployment",
        &duplicate_deployment_bindings,
        documents_by_id,
        &mut semantic_diagnostics,
    );
    push_duplicate_binding_diagnostics(
        "relationship",
        &duplicate_relationship_bindings,
        documents_by_id,
        &mut semantic_diagnostics,
    );

    WorkspaceBindingTables {
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
    references_by_target: BTreeMap<SymbolHandle, Vec<ReferenceHandle>>,
    semantic_diagnostics: Vec<SemanticDiagnostic>,
}

fn build_reference_resolution_tables(
    documents: &[DocumentId],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
    bindings: &WorkspaceBindingTables,
) -> WorkspaceReferenceTables {
    let mut reference_resolutions = BTreeMap::<ReferenceHandle, ReferenceResolutionStatus>::new();
    let mut references_by_target = BTreeMap::<SymbolHandle, Vec<ReferenceHandle>>::new();
    let mut semantic_diagnostics = Vec::new();

    for document_id in documents {
        let document = documents_by_id
            .get(document_id)
            .expect("BUG: workspace-index reference document should exist");
        let snapshot = document.snapshot();

        for (reference_index, reference) in snapshot.references().iter().enumerate() {
            let handle = ReferenceHandle {
                document: document_id.clone(),
                reference_index,
            };
            let status = resolve_reference_status(reference, bindings);

            if !snapshot.has_syntax_errors() {
                match status {
                    ReferenceResolutionStatus::UnresolvedNoMatch => {
                        semantic_diagnostics.push(SemanticDiagnostic::unresolved_reference(
                            document_id,
                            &reference.raw_text,
                            reference.span,
                        ));
                    }
                    ReferenceResolutionStatus::AmbiguousDuplicateBinding
                    | ReferenceResolutionStatus::AmbiguousElementVsRelationship => {
                        semantic_diagnostics.push(SemanticDiagnostic::ambiguous_reference(
                            document_id,
                            &reference.raw_text,
                            reference.span,
                        ));
                    }
                    ReferenceResolutionStatus::Resolved(_)
                    | ReferenceResolutionStatus::DeferredByScopePolicy => {}
                }
            }

            if let ReferenceResolutionStatus::Resolved(target) = &status {
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
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    for (key, handles) in duplicate_bindings {
        for handle in handles {
            let document = documents_by_id
                .get(handle.document())
                .expect("BUG: duplicate-binding document should exist");
            let snapshot = document.snapshot();
            if snapshot.has_syntax_errors() {
                continue;
            }

            let symbol = snapshot
                .symbols()
                .get(handle.symbol_id().0)
                .expect("BUG: duplicate-binding symbol should exist");
            diagnostics.push(SemanticDiagnostic::duplicate_binding(
                handle.document(),
                binding_kind,
                key,
                symbol.span,
            ));
        }
    }
}

fn resolve_reference_status(
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    // Syntax-role kinds remain useful to the LSP and diagnostics, but bounded
    // workspace resolution really depends on which binding family one reference
    // is allowed to target.
    match reference.kind {
        ReferenceKind::RelationshipSource
        | ReferenceKind::RelationshipDestination
        | ReferenceKind::InstanceTarget
        | ReferenceKind::DeploymentRelationshipSource
        | ReferenceKind::DeploymentRelationshipDestination
        | ReferenceKind::ViewScope
        | ReferenceKind::ViewInclude
        | ReferenceKind::ViewAnimation => {
            resolve_reference_against_target_hint(reference, bindings)
        }
    }
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
    snapshot: &crate::DocumentSnapshot,
    inherited_workspace_mode: Option<&IdentifierMode>,
) -> ElementIdentifierMode {
    match document_model_identifier_mode(snapshot)
        .or_else(|| document_workspace_identifier_mode(snapshot))
        .or_else(|| inherited_workspace_mode.cloned())
    {
        Some(IdentifierMode::Hierarchical) => ElementIdentifierMode::Hierarchical,
        Some(IdentifierMode::Flat) | None => ElementIdentifierMode::Flat,
        Some(IdentifierMode::Other(_)) => ElementIdentifierMode::Deferred,
    }
}

fn document_model_identifier_mode(snapshot: &crate::DocumentSnapshot) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(snapshot, &DirectiveContainer::Model)
}

fn document_workspace_identifier_mode(
    snapshot: &crate::DocumentSnapshot,
) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(snapshot, &DirectiveContainer::Workspace)
}

fn last_identifier_mode_for_container(
    snapshot: &crate::DocumentSnapshot,
    container: &DirectiveContainer,
) -> Option<IdentifierMode> {
    snapshot
        .identifier_modes()
        .iter()
        .rev()
        .find(|fact| fact.container == *container)
        .map(|fact| fact.mode.clone())
}

fn canonical_element_binding_key(
    snapshot: &crate::DocumentSnapshot,
    symbol_id: SymbolId,
    mode: ElementIdentifierMode,
) -> Option<String> {
    let symbol = snapshot.symbols().get(symbol_id.0)?;
    let binding_name = symbol.binding_name.as_deref()?;

    match mode {
        ElementIdentifierMode::Flat => Some(binding_name.to_owned()),
        ElementIdentifierMode::Deferred => None,
        ElementIdentifierMode::Hierarchical => {
            let mut segments = vec![binding_name.to_owned()];
            let mut parent = symbol.parent;

            while let Some(parent_id) = parent {
                let ancestor = snapshot.symbols().get(parent_id.0)?;
                if !matches!(
                    ancestor.kind,
                    SymbolKind::Person
                        | SymbolKind::SoftwareSystem
                        | SymbolKind::Container
                        | SymbolKind::Component
                ) {
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
) -> Vec<SemanticDiagnostic> {
    let mut diagnostic_counts = BTreeMap::<DocumentId, BTreeMap<SemanticDiagnostic, usize>>::new();

    for workspace_index in workspace_indexes {
        let mut per_document = BTreeMap::<DocumentId, BTreeSet<SemanticDiagnostic>>::new();

        for diagnostic in workspace_index.semantic_diagnostics() {
            per_document
                .entry(diagnostic.document.clone())
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

fn sort_semantic_diagnostics(diagnostics: &mut [SemanticDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.document
            .cmp(&right.document)
            .then_with(|| left.span.start_byte.cmp(&right.span.start_byte))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.message.cmp(&right.message))
    });
}

#[cfg(test)]
mod tests {
    use std::{
        fs, ptr,
        path::{Path, PathBuf},
    };

    use indoc::indoc;
    use tempfile::TempDir;

    use super::{WorkspaceLoader, document_id_from_path};

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
        assert!(ptr::eq(first_workspace.snapshot(), second_workspace.snapshot()));

        let first_model = first
            .document(&document_id_from_path(&fixture.model_path()))
            .expect("included model document should exist");
        let second_model = second
            .document(&document_id_from_path(&fixture.model_path()))
            .expect("included model document should exist");
        assert!(ptr::eq(first_model.snapshot(), second_model.snapshot()));
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
