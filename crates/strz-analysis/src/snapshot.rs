//! Immutable document inputs and snapshots used as the crate's main exchange objects.

use std::path::{Path, PathBuf};

use tree_sitter::Tree;

use crate::constants::ConstantDefinition;
use crate::diagnostics::RuledDiagnostic;
use crate::extract;
use crate::includes::{DirectiveValueKind, IncludeDirective};
use crate::semantic::{
    ConfigurationScopeFact, ElementDirectiveFact, PropertyFact, RelationshipFact,
    ResourceDirectiveFact, ViewFact, WorkspaceSectionFact,
};
use crate::symbols::{
    IdentifierModeFact, Reference, ReferenceKind, ReferenceTargetHint, Symbol, SymbolId, SymbolKind,
};
use crate::workspace::{
    ElementIdentifierMode, canonical_deployment_binding_key, canonical_element_binding_key,
    effective_element_identifier_mode_from_facts,
};

/// Stable caller-provided identifier for a document across analysis runs.
///
/// This is intentionally broader than a filesystem path. Workspace loading
/// commonly uses canonical paths, the LSP uses document URIs, and tests or
/// benchmarks sometimes use synthetic labels. When callers also have an on-disk
/// path, they should attach it separately via [`DocumentInput::with_location`].
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
    syntax_diagnostics: Vec<RuledDiagnostic>,
    include_directives: Vec<IncludeDirective>,
    constant_definitions: Vec<ConstantDefinition>,
    identifier_modes: Vec<IdentifierModeFact>,
    tags: Vec<String>,
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

impl DocumentSyntaxFacts {
    /// Extracts the stable syntax-level facts from one parsed document.
    pub(crate) fn collect(tree: &Tree, source: &str) -> Self {
        let syntax_diagnostics = extract::diagnostics::collect(tree);
        let include_directives = extract::includes::collect(tree, source);
        let constant_definitions = extract::constants::collect(tree, source);
        let identifier_modes = extract::symbols::collect_identifier_modes(tree, source);
        let tags = extract::symbols::collect_tags(tree, source);
        let (symbols, references) = extract::symbols::collect_symbols_and_references(tree, source);
        let semantic_facts = extract::semantic::collect(tree, source);

        Self {
            is_workspace_entry: contains_workspace_entry(tree),
            syntax_diagnostics,
            include_directives,
            constant_definitions,
            identifier_modes,
            tags,
            symbols,
            references,
            workspace_sections: semantic_facts.workspace_sections,
            configuration_scopes: semantic_facts.configuration_scopes,
            property_facts: semantic_facts.property_facts,
            resource_directives: semantic_facts.resource_directives,
            element_directives: semantic_facts.element_directives,
            relationship_facts: semantic_facts.relationship_facts,
            view_facts: semantic_facts.view_facts,
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
    pub fn syntax_diagnostics(&self) -> &[RuledDiagnostic] {
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

    /// Returns all normalized explicit tags observed anywhere in the document.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
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

    /// Returns top-level workspace sections such as `model` and `views`.
    #[must_use]
    pub fn workspace_sections(&self) -> &[WorkspaceSectionFact] {
        &self.workspace_sections
    }

    /// Returns all extracted `configuration { scope ... }` statements.
    #[must_use]
    pub fn configuration_scopes(&self) -> &[ConfigurationScopeFact] {
        &self.configuration_scopes
    }

    /// Returns all extracted `properties { ... }` entries.
    #[must_use]
    pub fn property_facts(&self) -> &[PropertyFact] {
        &self.property_facts
    }

    /// Returns all extracted `!docs` and `!adrs` directives.
    #[must_use]
    pub fn resource_directives(&self) -> &[ResourceDirectiveFact] {
        &self.resource_directives
    }

    /// Returns all extracted `!element` directive targets.
    #[must_use]
    pub fn element_directives(&self) -> &[ElementDirectiveFact] {
        &self.element_directives
    }

    /// Returns all extracted declared relationships.
    #[must_use]
    pub fn relationship_facts(&self) -> &[RelationshipFact] {
        &self.relationship_facts
    }

    /// Returns all extracted view definitions plus their body facts.
    #[must_use]
    pub fn view_facts(&self) -> &[ViewFact] {
        &self.view_facts
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
    pub fn syntax_diagnostics(&self) -> &[RuledDiagnostic] {
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
    /// Returns all normalized explicit tags observed anywhere in the document.
    pub fn tags(&self) -> &[String] {
        self.syntax_facts.tags()
    }

    #[must_use]
    /// Returns the document's effective bounded element-identifier mode.
    pub fn effective_element_identifier_mode(&self) -> ElementIdentifierMode {
        self.effective_element_identifier_mode_with(None)
    }

    #[must_use]
    /// Returns the document's effective bounded element-identifier mode, optionally
    /// using a caller-provided inherited workspace mode when one is already known.
    pub fn effective_element_identifier_mode_with(
        &self,
        inherited_workspace_mode: Option<ElementIdentifierMode>,
    ) -> ElementIdentifierMode {
        inherited_workspace_mode.unwrap_or_else(|| {
            effective_element_identifier_mode_from_facts(self.identifier_modes(), None)
        })
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

    #[must_use]
    /// Resolves one extracted reference against the current document only.
    pub fn resolve_reference(&self, reference: &Reference) -> Option<&Symbol> {
        self.resolve_reference_with_mode(reference, self.effective_element_identifier_mode())
    }

    #[must_use]
    /// Resolves one extracted reference against the current document using an
    /// explicit effective identifier mode when the caller already knows the
    /// inherited workspace policy for this document.
    pub fn resolve_reference_with_mode(
        &self,
        reference: &Reference,
        mode: ElementIdentifierMode,
    ) -> Option<&Symbol> {
        if is_contextual_this_reference(reference) {
            return resolve_contextual_this_reference(self, reference, mode).resolved();
        }

        match resolve_reference_raw_text(self, reference.target_hint, &reference.raw_text, mode) {
            SnapshotReferenceResolution::Resolved(symbol) => Some(symbol),
            SnapshotReferenceResolution::Ambiguous => None,
            SnapshotReferenceResolution::Unresolved => {
                match resolve_reference_with_symbol_context(self, reference, mode) {
                    SnapshotReferenceResolution::Resolved(symbol) => Some(symbol),
                    SnapshotReferenceResolution::Ambiguous => None,
                    SnapshotReferenceResolution::Unresolved => {
                        resolve_reference_with_selector_context(self, reference, mode).resolved()
                    }
                }
            }
        }
    }

    #[must_use]
    /// Returns top-level workspace sections such as `model` and `views`.
    pub fn workspace_sections(&self) -> &[WorkspaceSectionFact] {
        self.syntax_facts.workspace_sections()
    }

    #[must_use]
    /// Returns all extracted `configuration { scope ... }` statements.
    pub fn configuration_scopes(&self) -> &[ConfigurationScopeFact] {
        self.syntax_facts.configuration_scopes()
    }

    #[must_use]
    /// Returns all extracted `properties { ... }` entries.
    pub fn property_facts(&self) -> &[PropertyFact] {
        self.syntax_facts.property_facts()
    }

    #[must_use]
    /// Returns all extracted `!docs` and `!adrs` directives.
    pub fn resource_directives(&self) -> &[ResourceDirectiveFact] {
        self.syntax_facts.resource_directives()
    }

    #[must_use]
    /// Returns all extracted `!element` directive targets.
    pub fn element_directives(&self) -> &[ElementDirectiveFact] {
        self.syntax_facts.element_directives()
    }

    #[must_use]
    /// Returns all extracted declared relationships.
    pub fn relationship_facts(&self) -> &[RelationshipFact] {
        self.syntax_facts.relationship_facts()
    }

    #[must_use]
    /// Returns all extracted view definitions plus their body facts.
    pub fn view_facts(&self) -> &[ViewFact] {
        self.syntax_facts.view_facts()
    }
}

fn contains_workspace_entry(tree: &Tree) -> bool {
    let root = tree.root_node();
    let mut cursor = root.walk();

    root.named_children(&mut cursor)
        .any(|child| matches!(child.kind(), "workspace" | "workspace_block"))
}

#[derive(Clone, Copy)]
enum SnapshotContextualOwnerTarget {
    Element,
    Deployment,
}

#[derive(Debug, Clone, Copy)]
enum SnapshotReferenceResolution<'a> {
    Resolved(&'a Symbol),
    Unresolved,
    Ambiguous,
}

impl<'a> SnapshotReferenceResolution<'a> {
    const fn resolved(self) -> Option<&'a Symbol> {
        match self {
            Self::Resolved(symbol) => Some(symbol),
            Self::Unresolved | Self::Ambiguous => None,
        }
    }
}

fn is_contextual_this_reference(reference: &Reference) -> bool {
    reference.raw_text == "this"
        && matches!(
            reference.kind,
            ReferenceKind::RelationshipSource
                | ReferenceKind::RelationshipDestination
                | ReferenceKind::DeploymentRelationshipSource
                | ReferenceKind::DeploymentRelationshipDestination
        )
}

fn resolve_contextual_this_reference<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
    mode: ElementIdentifierMode,
) -> SnapshotReferenceResolution<'a> {
    match resolve_selector_owner_target(snapshot, reference, mode) {
        Some(SnapshotReferenceResolution::Resolved(symbol)) => {
            return SnapshotReferenceResolution::Resolved(symbol);
        }
        Some(SnapshotReferenceResolution::Ambiguous) => {
            return SnapshotReferenceResolution::Ambiguous;
        }
        Some(SnapshotReferenceResolution::Unresolved) => {
            return SnapshotReferenceResolution::Unresolved;
        }
        None => {}
    }

    let start_symbol = reference
        .containing_symbol
        .or_else(|| enclosing_symbol_for_span(snapshot, reference.span));
    contextual_symbol_target(snapshot, start_symbol, reference)
}

fn resolve_selector_owner_target<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
    mode: ElementIdentifierMode,
) -> Option<SnapshotReferenceResolution<'a>> {
    let selector_target = enclosing_element_selector_target(snapshot, reference.span)?;
    if !matches!(
        selector_target.value_kind,
        DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
    ) {
        return Some(SnapshotReferenceResolution::Unresolved);
    }

    let target_hint = match reference.target_hint {
        ReferenceTargetHint::Element => SnapshotContextualOwnerTarget::Element,
        ReferenceTargetHint::Deployment => SnapshotContextualOwnerTarget::Deployment,
        ReferenceTargetHint::ElementOrDeployment
        | ReferenceTargetHint::Relationship
        | ReferenceTargetHint::ElementOrRelationship => {
            return Some(SnapshotReferenceResolution::Unresolved);
        }
    };

    let directive = enclosing_element_directive(snapshot, reference.span)?;
    for candidate in selector_target_candidates(snapshot, directive, mode) {
        let resolution = match target_hint {
            SnapshotContextualOwnerTarget::Element => {
                resolve_reference_raw_text(snapshot, ReferenceTargetHint::Element, &candidate, mode)
            }
            SnapshotContextualOwnerTarget::Deployment => resolve_reference_raw_text(
                snapshot,
                ReferenceTargetHint::Deployment,
                &candidate,
                mode,
            ),
        };
        if !matches!(resolution, SnapshotReferenceResolution::Unresolved) {
            return Some(resolution);
        }
    }

    Some(SnapshotReferenceResolution::Unresolved)
}

fn contextual_symbol_target<'a>(
    snapshot: &'a DocumentSnapshot,
    start_symbol: Option<SymbolId>,
    reference: &Reference,
) -> SnapshotReferenceResolution<'a> {
    let matches_kind: fn(SymbolKind) -> bool = match reference.target_hint {
        ReferenceTargetHint::Element => is_element_symbol_kind,
        ReferenceTargetHint::Deployment => is_deployment_symbol_kind,
        ReferenceTargetHint::ElementOrDeployment
        | ReferenceTargetHint::Relationship
        | ReferenceTargetHint::ElementOrRelationship => {
            return SnapshotReferenceResolution::Unresolved;
        }
    };

    let mut current = start_symbol;
    while let Some(symbol_id) = current {
        let Some(symbol) = snapshot.symbols().get(symbol_id.0) else {
            return SnapshotReferenceResolution::Unresolved;
        };
        if matches_kind(symbol.kind) {
            return SnapshotReferenceResolution::Resolved(symbol);
        }
        current = symbol.parent;
    }

    SnapshotReferenceResolution::Unresolved
}

fn resolve_reference_with_symbol_context<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
    mode: ElementIdentifierMode,
) -> SnapshotReferenceResolution<'a> {
    let Some(containing_symbol) = reference.containing_symbol else {
        return SnapshotReferenceResolution::Unresolved;
    };
    for prefix in
        contextual_reference_prefixes(snapshot, containing_symbol, mode, reference.target_hint)
    {
        let contextual_raw_text = format!("{prefix}.{}", reference.raw_text);
        let resolution =
            resolve_reference_raw_text(snapshot, reference.target_hint, &contextual_raw_text, mode);
        if !matches!(resolution, SnapshotReferenceResolution::Unresolved) {
            return resolution;
        }
    }

    SnapshotReferenceResolution::Unresolved
}

fn resolve_reference_with_selector_context<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
    mode: ElementIdentifierMode,
) -> SnapshotReferenceResolution<'a> {
    let Some(directive) = enclosing_element_directive(snapshot, reference.span) else {
        return SnapshotReferenceResolution::Unresolved;
    };
    if !matches!(
        directive.target.value_kind,
        DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
    ) {
        return SnapshotReferenceResolution::Unresolved;
    }

    for selector_target in selector_target_candidates(snapshot, directive, mode) {
        let contextual_raw_text = format!("{selector_target}.{}", reference.raw_text);
        let resolution =
            resolve_reference_raw_text(snapshot, reference.target_hint, &contextual_raw_text, mode);
        if !matches!(resolution, SnapshotReferenceResolution::Unresolved) {
            return resolution;
        }
    }

    SnapshotReferenceResolution::Unresolved
}

fn resolve_reference_raw_text<'a>(
    snapshot: &'a DocumentSnapshot,
    target_hint: ReferenceTargetHint,
    raw_text: &str,
    mode: ElementIdentifierMode,
) -> SnapshotReferenceResolution<'a> {
    let mut candidates = snapshot.symbols().iter().filter(|symbol| {
        symbol_matches_reference_raw_text(snapshot, symbol, target_hint, raw_text, mode)
    });
    let Some(first) = candidates.next() else {
        return SnapshotReferenceResolution::Unresolved;
    };
    if candidates.next().is_some() {
        return SnapshotReferenceResolution::Ambiguous;
    }

    SnapshotReferenceResolution::Resolved(first)
}

fn symbol_matches_reference_raw_text(
    snapshot: &DocumentSnapshot,
    symbol: &Symbol,
    target_hint: ReferenceTargetHint,
    raw_text: &str,
    mode: ElementIdentifierMode,
) -> bool {
    let relationship_match =
        symbol.kind == SymbolKind::Relationship && symbol.binding_name.as_deref() == Some(raw_text);
    let element_match = canonical_element_binding_key(snapshot.symbols(), symbol.id, mode)
        .as_deref()
        == Some(raw_text);
    let deployment_match = canonical_deployment_binding_key(snapshot.symbols(), symbol.id, mode)
        .as_deref()
        == Some(raw_text);

    match target_hint {
        ReferenceTargetHint::Element => element_match,
        ReferenceTargetHint::ElementOrDeployment => element_match || deployment_match,
        ReferenceTargetHint::Deployment => deployment_match,
        ReferenceTargetHint::Relationship => relationship_match,
        ReferenceTargetHint::ElementOrRelationship => element_match || relationship_match,
    }
}

fn contextual_reference_prefixes(
    snapshot: &DocumentSnapshot,
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
    target_hint: ReferenceTargetHint,
) -> Vec<String> {
    match target_hint {
        ReferenceTargetHint::Element => contextual_prefixes(
            snapshot,
            containing_symbol,
            mode,
            &[ReferenceTargetHint::Element],
        ),
        ReferenceTargetHint::ElementOrDeployment => contextual_prefixes(
            snapshot,
            containing_symbol,
            mode,
            &[
                ReferenceTargetHint::Element,
                ReferenceTargetHint::Deployment,
            ],
        ),
        ReferenceTargetHint::Deployment => contextual_prefixes(
            snapshot,
            containing_symbol,
            mode,
            &[ReferenceTargetHint::Deployment],
        ),
        ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => {
            Vec::new()
        }
    }
}

fn contextual_prefixes(
    snapshot: &DocumentSnapshot,
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
    target_hints: &[ReferenceTargetHint],
) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut current = Some(containing_symbol);

    while let Some(symbol_id) = current {
        let Some(symbol) = snapshot.symbols().get(symbol_id.0) else {
            break;
        };
        for target_hint in target_hints {
            let candidate = match target_hint {
                ReferenceTargetHint::Element => {
                    canonical_element_binding_key(snapshot.symbols(), symbol_id, mode)
                }
                ReferenceTargetHint::Deployment => {
                    canonical_deployment_binding_key(snapshot.symbols(), symbol_id, mode)
                }
                ReferenceTargetHint::ElementOrDeployment
                | ReferenceTargetHint::Relationship
                | ReferenceTargetHint::ElementOrRelationship => None,
            };
            if let Some(prefix) = candidate {
                prefixes.push(prefix);
            }
        }
        current = symbol.parent;
    }

    prefixes
}

fn enclosing_element_directive(
    snapshot: &DocumentSnapshot,
    span: crate::TextSpan,
) -> Option<&ElementDirectiveFact> {
    snapshot
        .element_directives()
        .iter()
        .filter(|directive| span_within(directive.span, span))
        .min_by_key(|directive| directive.span.end_byte - directive.span.start_byte)
}

fn enclosing_element_selector_target(
    snapshot: &DocumentSnapshot,
    span: crate::TextSpan,
) -> Option<&crate::ValueFact> {
    enclosing_element_directive(snapshot, span).map(|directive| &directive.target)
}

fn selector_target_candidates(
    snapshot: &DocumentSnapshot,
    directive: &ElementDirectiveFact,
    mode: ElementIdentifierMode,
) -> Vec<String> {
    let raw_text = directive.target.normalized_text.as_str();
    let mut candidates = vec![raw_text.to_owned()];
    let Some(containing_symbol) = enclosing_symbol_for_span(snapshot, directive.span) else {
        return candidates;
    };

    for prefix in contextual_prefixes(
        snapshot,
        containing_symbol,
        mode,
        &[
            ReferenceTargetHint::Element,
            ReferenceTargetHint::Deployment,
        ],
    ) {
        candidates.push(format!("{prefix}.{raw_text}"));
    }

    candidates
}

fn enclosing_symbol_for_span(
    snapshot: &DocumentSnapshot,
    span: crate::TextSpan,
) -> Option<SymbolId> {
    snapshot
        .symbols()
        .iter()
        .filter(|symbol| span_within(symbol.span, span))
        .min_by_key(|symbol| symbol.span.end_byte - symbol.span.start_byte)
        .map(|symbol| symbol.id)
}

const fn is_element_symbol_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Person
            | SymbolKind::SoftwareSystem
            | SymbolKind::Container
            | SymbolKind::Component
    )
}

const fn is_deployment_symbol_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::DeploymentEnvironment
            | SymbolKind::DeploymentNode
            | SymbolKind::InfrastructureNode
            | SymbolKind::ContainerInstance
            | SymbolKind::SoftwareSystemInstance
    )
}

const fn span_within(outer: crate::TextSpan, inner: crate::TextSpan) -> bool {
    outer.start_byte <= inner.start_byte && inner.end_byte <= outer.end_byte
}
