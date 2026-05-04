// Public workspace-facing data types and the private semantic packets that the
// workspace layer reuses across discovery, indexing, and diagnostics.

// Public workspace-facing data types and the private semantic packets that the
// workspace layer reuses across discovery, indexing, and diagnostics.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use ignore::WalkBuilder;

use crate::{
    Annotation, ConstantDefinition, DocumentAnalyzer, DocumentId, DocumentInput, DocumentLocation,
    IdentifierMode, IdentifierModeFact, IncludeDirective, Reference, ReferenceKind,
    ReferenceTargetHint, RuleId, RuledDiagnostic, Symbol, SymbolId, SymbolKind, TextSpan,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceDirectiveEvent {
    ConstantDefinition(usize),
    IncludeDirective(usize),
}

/// One discovered document plus the metadata gathered during workspace loading.
#[derive(Debug)]
pub struct WorkspaceDocument {
    snapshot: Arc<crate::DocumentSnapshot>,
    directive_events: Arc<[WorkspaceDirectiveEvent]>,
    semantic_facts: Arc<WorkspaceSemanticDocumentFacts>,
    kind: WorkspaceDocumentKind,
    semantic_generation: u64,
    discovered_by_scan: bool,
}

impl WorkspaceDocument {
    const fn new(
        snapshot: Arc<crate::DocumentSnapshot>,
        directive_events: Arc<[WorkspaceDirectiveEvent]>,
        semantic_facts: Arc<WorkspaceSemanticDocumentFacts>,
        kind: WorkspaceDocumentKind,
        semantic_generation: u64,
        discovered_by_scan: bool,
    ) -> Self {
        Self {
            snapshot,
            directive_events,
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

    fn snapshot_handle(&self) -> Arc<crate::DocumentSnapshot> {
        Arc::clone(&self.snapshot)
    }

    fn directive_events_handle(&self) -> Arc<[WorkspaceDirectiveEvent]> {
        Arc::clone(&self.directive_events)
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

/// Per-instance symbol projection for workspace-wide symbol queries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceSymbolFact {
    canonical_key: String,
    root_document: DocumentId,
    source_document: DocumentId,
    handle: SymbolHandle,
}

impl WorkspaceSymbolFact {
    /// Creates a symbol fact scoped to one derived workspace instance.
    #[must_use]
    pub const fn new(
        canonical_key: String,
        root_document: DocumentId,
        source_document: DocumentId,
        handle: SymbolHandle,
    ) -> Self {
        Self {
            canonical_key,
            root_document,
            source_document,
            handle,
        }
    }

    /// Returns the canonical key that identifies this symbol inside the instance.
    #[must_use]
    pub fn canonical_key(&self) -> &str {
        &self.canonical_key
    }

    /// Returns the root document that supplied this symbol's semantic context.
    #[must_use]
    pub const fn root_document(&self) -> &DocumentId {
        &self.root_document
    }

    /// Returns the document that contains the symbol declaration.
    #[must_use]
    pub const fn source_document(&self) -> &DocumentId {
        &self.source_document
    }

    /// Returns the stable handle for the declaration symbol.
    #[must_use]
    pub const fn handle(&self) -> &SymbolHandle {
        &self.handle
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
    workspace_symbols: Vec<WorkspaceSymbolFact>,
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
    document_location: Option<DocumentLocation>,
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
            document_location: snapshot.location().cloned(),
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

    /// Returns the per-instance symbols available to workspace-wide symbol queries.
    #[must_use]
    pub fn workspace_symbols(&self) -> &[WorkspaceSymbolFact] {
        &self.derived.workspace_symbols
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

    /// Returns the unique canonical binding key for one symbol, if this
    /// workspace instance can name it without ambiguity.
    #[must_use]
    pub fn unique_binding_key_for_symbol(&self, handle: &SymbolHandle) -> Option<&str> {
        self.derived
            .unique_element_bindings
            .iter()
            .chain(self.derived.unique_deployment_bindings.iter())
            .chain(self.derived.unique_relationship_bindings.iter())
            .find_map(|(key, candidate)| (candidate == handle).then_some(key.as_str()))
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

#[cfg(test)]
mod workspace_model_tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn workspace_document_caches_directive_event_order_for_context_replay() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                !include "a.dsl"
                !constant env dev
                !constant region eu-west-1
                !include "b.dsl"
            }
        "#});
        fixture.write_file("a.dsl", "model {}");
        fixture.write_file("b.dsl", "views {}");

        let workspace = WorkspaceLoader::new()
            .load_paths([fixture.root()])
            .expect("workspace should load");
        let document = workspace
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");

        assert_eq!(
            document.directive_events_handle().as_ref(),
            &[
                WorkspaceDirectiveEvent::IncludeDirective(0),
                WorkspaceDirectiveEvent::ConstantDefinition(0),
                WorkspaceDirectiveEvent::ConstantDefinition(1),
                WorkspaceDirectiveEvent::IncludeDirective(1),
            ]
        );
    }
}
