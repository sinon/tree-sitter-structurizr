//! Shared navigation helpers for same-document fallback and workspace indexing.

use line_index::LineIndex;
use strz_analysis::{
    DirectiveValueKind, DocumentId, DocumentLocation, DocumentSnapshot, Reference, ReferenceHandle,
    ReferenceKind, ReferenceResolutionStatus, Symbol, SymbolHandle, SymbolId, SymbolKind,
    WorkspaceFacts, WorkspaceInstanceId,
};
use tower_lsp_server::ls_types::Location;
use tracing::debug;

use crate::{
    convert::{positions::span_to_range, uris::file_uri_from_path},
    documents::DocumentState,
    state::ServerState,
};

pub enum NavigationSite<'a> {
    Symbol(&'a Symbol),
    Reference {
        index: usize,
        reference: &'a Reference,
    },
}

/// Finds the declaration or reference site at one byte offset.
pub fn navigation_site_at_offset(
    snapshot: &DocumentSnapshot,
    offset: usize,
) -> Option<NavigationSite<'_>> {
    reference_at_offset(snapshot, offset).map_or_else(
        || bindable_symbol_at_offset(snapshot, offset).map(NavigationSite::Symbol),
        |(index, reference)| Some(NavigationSite::Reference { index, reference }),
    )
}

/// Finds the declaration or reference target at one byte offset.
///
/// Returns the referenced declaration when the offset lands on a reference, or
/// the directly bound declaration when the offset lands on a symbol site.
pub fn target_symbol_at_offset(snapshot: &DocumentSnapshot, offset: usize) -> Option<&Symbol> {
    navigation_site_at_offset(snapshot, offset).and_then(|site| match site {
        NavigationSite::Symbol(symbol) => Some(symbol),
        NavigationSite::Reference { reference, .. } => resolve_reference(snapshot, reference),
    })
}

/// Resolves the declaration symbol that should back one hover or other read-only
/// identifier request.
pub fn resolved_symbol_at_offset<'a>(
    state: &'a ServerState,
    document: &'a DocumentState,
    snapshot: &'a DocumentSnapshot,
    offset: usize,
) -> Option<&'a Symbol> {
    match navigation_site_at_offset(snapshot, offset)? {
        NavigationSite::Symbol(symbol) => Some(symbol),
        NavigationSite::Reference { index, reference } => {
            if let Some((workspace_facts, document_id)) = workspace_context(state, document) {
                let candidate_instances = candidate_instances(workspace_facts, &document_id);
                if !candidate_instances.is_empty() {
                    let reference_handle = ReferenceHandle::new(document_id, index);
                    let target = unanimous_resolved_symbol(
                        workspace_facts,
                        &candidate_instances,
                        &reference_handle,
                    )?;

                    return workspace_facts
                        .document(target.document())?
                        .snapshot()
                        .symbols()
                        .get(target.symbol_id().0);
                }
            }

            resolve_reference(snapshot, reference)
        }
    }
}

/// Collects all same-document references that resolve to one symbol.
#[must_use]
pub fn references_for_symbol<'a>(
    snapshot: &'a DocumentSnapshot,
    symbol: &Symbol,
) -> Vec<&'a Reference> {
    snapshot
        .references()
        .iter()
        .filter(|reference| resolve_reference(snapshot, reference) == Some(symbol))
        .collect()
}

pub fn definition_location(
    state: &ServerState,
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    offset: usize,
) -> Option<Location> {
    match navigation_site_at_offset(snapshot, offset)? {
        NavigationSite::Symbol(symbol) => same_document_symbol_location(document, symbol),
        NavigationSite::Reference { index, reference } => {
            if let Some((workspace_facts, document_id)) = workspace_context(state, document) {
                let candidate_instances = candidate_instances(workspace_facts, &document_id);
                if !candidate_instances.is_empty() {
                    debug!(
                        uri = document.uri().as_str(),
                        offset,
                        reference = %reference.raw_text,
                        candidate_instance_count = candidate_instances.len(),
                        "attempting workspace-aware gotoDefinition resolution"
                    );
                    let reference_handle = ReferenceHandle::new(document_id, index);
                    let target = unanimous_resolved_symbol(
                        workspace_facts,
                        &candidate_instances,
                        &reference_handle,
                    )?;

                    debug!(
                        uri = document.uri().as_str(),
                        offset,
                        ?target,
                        "gotoDefinition resolved through workspace index"
                    );
                    return symbol_location(state, workspace_facts, &target);
                }
            }

            debug!(
                uri = document.uri().as_str(),
                offset,
                reference = %reference.raw_text,
                "falling back to same-document gotoDefinition resolution"
            );
            resolve_reference(snapshot, reference)
                .and_then(|symbol| same_document_symbol_location(document, symbol))
        }
    }
}

pub fn type_definition_location(
    state: &ServerState,
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    offset: usize,
) -> Option<Location> {
    match navigation_site_at_offset(snapshot, offset)? {
        NavigationSite::Symbol(symbol) => {
            if let Some((workspace_facts, document_id)) = workspace_context(state, document) {
                let candidate_instances = candidate_instances(workspace_facts, &document_id);
                if !candidate_instances.is_empty() {
                    let symbol_handle = SymbolHandle::new(document_id, symbol.id);
                    let target = instance_type_symbol_handle(workspace_facts, &symbol_handle)?;
                    return symbol_location(state, workspace_facts, &target);
                }
            }

            instance_type_symbol(snapshot, symbol)
                .and_then(|target| same_document_symbol_location(document, target))
        }
        NavigationSite::Reference { index, reference } => {
            if let Some((workspace_facts, document_id)) = workspace_context(state, document) {
                let candidate_instances = candidate_instances(workspace_facts, &document_id);
                if !candidate_instances.is_empty() {
                    let reference_handle = ReferenceHandle::new(document_id, index);
                    let target = instance_type_symbol_handle_for_reference(
                        workspace_facts,
                        &candidate_instances,
                        &reference_handle,
                        reference,
                    )?;
                    return symbol_location(state, workspace_facts, &target);
                }
            }

            instance_type_symbol_for_reference(snapshot, reference)
                .and_then(|target| same_document_symbol_location(document, target))
        }
    }
}

#[must_use]
pub fn reference_locations(
    state: &ServerState,
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    offset: usize,
    include_declaration: bool,
) -> Vec<Location> {
    let Some(site) = navigation_site_at_offset(snapshot, offset) else {
        return Vec::new();
    };

    if let Some((workspace_facts, document_id)) = workspace_context(state, document) {
        let candidate_instances = candidate_instances(workspace_facts, &document_id);
        if !candidate_instances.is_empty() {
            debug!(
                uri = document.uri().as_str(),
                offset,
                include_declaration,
                candidate_instance_count = candidate_instances.len(),
                "attempting workspace-aware references resolution"
            );
            let workspace_locations = match site {
                NavigationSite::Symbol(symbol) => {
                    let symbol_handle = SymbolHandle::new(document_id, symbol.id);
                    let reference_handles = unanimous_reference_handles(
                        workspace_facts,
                        &candidate_instances,
                        &symbol_handle,
                    );

                    reference_handles.map(|reference_handles| {
                        materialize_reference_locations(
                            state,
                            workspace_facts,
                            include_declaration,
                            &symbol_handle,
                            &reference_handles,
                        )
                    })
                }
                NavigationSite::Reference { index, .. } => {
                    let reference_handle = ReferenceHandle::new(document_id, index);
                    let symbol_handle = unanimous_resolved_symbol(
                        workspace_facts,
                        &candidate_instances,
                        &reference_handle,
                    );
                    let reference_handles = symbol_handle.as_ref().and_then(|symbol_handle| {
                        unanimous_reference_handles(
                            workspace_facts,
                            &candidate_instances,
                            symbol_handle,
                        )
                    });

                    symbol_handle.zip(reference_handles).map(
                        |(symbol_handle, reference_handles)| {
                            materialize_reference_locations(
                                state,
                                workspace_facts,
                                include_declaration,
                                &symbol_handle,
                                &reference_handles,
                            )
                        },
                    )
                }
            };

            debug!(
                uri = document.uri().as_str(),
                offset,
                include_declaration,
                location_count = workspace_locations.as_ref().map_or(0, Vec::len),
                "workspace-aware references resolution completed"
            );
            return workspace_locations.unwrap_or_default();
        }
    }

    let Some(symbol) = target_symbol_at_offset(snapshot, offset) else {
        return Vec::new();
    };

    let mut locations = Vec::new();
    if include_declaration && let Some(location) = same_document_symbol_location(document, symbol) {
        locations.push(location);
    }

    locations.extend(
        references_for_symbol(snapshot, symbol)
            .into_iter()
            .filter_map(|reference| same_document_reference_location(document, reference)),
    );
    locations
}

fn instance_type_symbol_handle(
    workspace_facts: &WorkspaceFacts,
    symbol_handle: &SymbolHandle,
) -> Option<SymbolHandle> {
    let snapshot = workspace_facts
        .document(symbol_handle.document())?
        .snapshot();
    let symbol = snapshot.symbols().get(symbol_handle.symbol_id().0)?;
    if !is_instance_symbol(symbol) {
        return None;
    }

    let (reference_index, _) = instance_target_reference(snapshot, symbol.id)?;
    let candidate_instances = candidate_instances(workspace_facts, symbol_handle.document());
    if candidate_instances.is_empty() {
        return None;
    }

    let reference_handle = ReferenceHandle::new(symbol_handle.document().clone(), reference_index);
    unanimous_resolved_symbol(workspace_facts, &candidate_instances, &reference_handle)
}

fn instance_type_symbol_handle_for_reference(
    workspace_facts: &WorkspaceFacts,
    candidate_instances: &[WorkspaceInstanceId],
    reference_handle: &ReferenceHandle,
    reference: &Reference,
) -> Option<SymbolHandle> {
    if reference.kind == ReferenceKind::InstanceTarget {
        return unanimous_resolved_symbol(workspace_facts, candidate_instances, reference_handle);
    }

    let symbol_handle =
        unanimous_resolved_symbol(workspace_facts, candidate_instances, reference_handle)?;
    instance_type_symbol_handle(workspace_facts, &symbol_handle)
}

fn candidate_instances(
    workspace_facts: &WorkspaceFacts,
    document_id: &DocumentId,
) -> Vec<WorkspaceInstanceId> {
    workspace_facts
        .candidate_instances_for(document_id)
        .copied()
        .collect()
}

pub(super) fn unanimous_resolved_symbol(
    workspace_facts: &WorkspaceFacts,
    candidate_instances: &[WorkspaceInstanceId],
    reference_handle: &ReferenceHandle,
) -> Option<SymbolHandle> {
    let mut resolved_symbol = None;

    for instance_id in candidate_instances {
        let status = workspace_facts
            .workspace_index(*instance_id)?
            .reference_resolution(reference_handle)?;
        let ReferenceResolutionStatus::Resolved(symbol_handle) = status else {
            debug!(
                instance_id = instance_id.as_usize(),
                ?reference_handle,
                ?status,
                "workspace instance could not resolve one reference unanimously"
            );
            return None;
        };

        if resolved_symbol
            .as_ref()
            .is_some_and(|existing| existing != symbol_handle)
        {
            debug!(
                instance_id = instance_id.as_usize(),
                ?reference_handle,
                ?symbol_handle,
                existing = ?resolved_symbol,
                "workspace instances disagreed on one resolved definition target"
            );
            return None;
        }

        resolved_symbol = Some(symbol_handle.clone());
    }

    resolved_symbol
}

fn unanimous_reference_handles(
    workspace_facts: &WorkspaceFacts,
    candidate_instances: &[WorkspaceInstanceId],
    symbol_handle: &SymbolHandle,
) -> Option<Vec<ReferenceHandle>> {
    let mut first = None;

    for instance_id in candidate_instances {
        let mut references = workspace_facts
            .workspace_index(*instance_id)?
            .references_for_symbol(symbol_handle)
            .cloned()
            .collect::<Vec<_>>();
        references.sort();
        references.dedup();

        if first
            .as_ref()
            .is_some_and(|existing| existing != &references)
        {
            debug!(
                instance_id = instance_id.as_usize(),
                ?symbol_handle,
                ?references,
                existing = ?first,
                "workspace instances disagreed on one references result set"
            );
            return None;
        }

        first = Some(references);
    }

    first
}

fn materialize_reference_locations(
    state: &ServerState,
    workspace_facts: &WorkspaceFacts,
    include_declaration: bool,
    symbol_handle: &SymbolHandle,
    reference_handles: &[ReferenceHandle],
) -> Vec<Location> {
    let mut locations = Vec::new();

    if include_declaration
        && let Some(location) = symbol_location(state, workspace_facts, symbol_handle)
    {
        locations.push(location);
    }

    locations.extend(
        reference_handles
            .iter()
            .filter_map(|handle| reference_location(state, workspace_facts, handle)),
    );
    locations
}

fn symbol_location(
    state: &ServerState,
    workspace_facts: &WorkspaceFacts,
    symbol_handle: &SymbolHandle,
) -> Option<Location> {
    let document = open_document_by_id(state, symbol_handle.document());
    let snapshot = workspace_facts
        .document(symbol_handle.document())?
        .snapshot();
    let symbol = snapshot.symbols().get(symbol_handle.symbol_id().0)?;

    location_for_span(
        document,
        snapshot.location(),
        snapshot.source(),
        symbol_navigation_span(symbol),
    )
}

fn reference_location(
    state: &ServerState,
    workspace_facts: &WorkspaceFacts,
    reference_handle: &ReferenceHandle,
) -> Option<Location> {
    let document = open_document_by_id(state, reference_handle.document());
    let snapshot = workspace_facts
        .document(reference_handle.document())?
        .snapshot();
    let reference = snapshot
        .references()
        .get(reference_handle.reference_index())?;

    location_for_span(
        document,
        snapshot.location(),
        snapshot.source(),
        reference.span,
    )
}

fn location_for_span(
    open_document: Option<&DocumentState>,
    location: Option<&DocumentLocation>,
    source: &str,
    span: strz_analysis::TextSpan,
) -> Option<Location> {
    if let Some(document) = open_document {
        let range = span_to_range(document.line_index(), span)?;
        return Some(Location::new(document.uri().clone(), range));
    }

    let line_index = LineIndex::new(source);
    let range = span_to_range(&line_index, span)?;
    let uri = file_uri_from_path(location?.path())?;
    Some(Location::new(uri, range))
}

fn same_document_symbol_location(document: &DocumentState, symbol: &Symbol) -> Option<Location> {
    let range = span_to_range(document.line_index(), symbol_navigation_span(symbol))?;
    Some(Location::new(document.uri().clone(), range))
}

fn symbol_navigation_span(symbol: &Symbol) -> strz_analysis::TextSpan {
    symbol.binding_span.unwrap_or(symbol.span)
}

fn same_document_reference_location(
    document: &DocumentState,
    reference: &Reference,
) -> Option<Location> {
    let range = span_to_range(document.line_index(), reference.span)?;
    Some(Location::new(document.uri().clone(), range))
}

fn workspace_context<'a>(
    state: &'a ServerState,
    document: &DocumentState,
) -> Option<(&'a WorkspaceFacts, DocumentId)> {
    Some((state.workspace_facts()?, workspace_document_id(document)?))
}

fn open_document_by_id<'a>(
    state: &'a ServerState,
    document_id: &DocumentId,
) -> Option<&'a DocumentState> {
    state.documents().iter().find(|document| {
        workspace_document_id(document)
            .as_ref()
            .is_some_and(|candidate| candidate == document_id)
    })
}

fn workspace_document_id(document: &DocumentState) -> Option<DocumentId> {
    document.workspace_document_id().cloned()
}

fn bindable_symbol_at_offset(snapshot: &DocumentSnapshot, offset: usize) -> Option<&Symbol> {
    snapshot
        .symbols()
        .iter()
        .filter(|symbol| {
            symbol
                .binding_span
                .is_some_and(|span| span_contains(span.start_byte, span.end_byte, offset))
        })
        .min_by_key(|symbol| {
            let span = symbol
                .binding_span
                .expect("binding span should exist for bindable symbol");
            span.end_byte - span.start_byte
        })
}

fn reference_at_offset(snapshot: &DocumentSnapshot, offset: usize) -> Option<(usize, &Reference)> {
    snapshot
        .references()
        .iter()
        .enumerate()
        .find(|(_, reference)| {
            span_contains(reference.span.start_byte, reference.span.end_byte, offset)
        })
}

pub(super) fn resolve_reference<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
) -> Option<&'a Symbol> {
    if is_contextual_this_reference(reference) {
        return resolve_contextual_this_reference(snapshot, reference);
    }

    // Prefer returning no result over guessing between multiple candidates. The
    // same-document fallback stays conservative when workspace indexes are not
    // available or when the current file is not part of a known workspace.
    let candidates: Vec<&Symbol> = snapshot
        .symbols()
        .iter()
        .filter(|symbol| symbol_matches_reference(symbol, reference))
        .collect();

    if candidates.len() == 1 {
        candidates.into_iter().next()
    } else {
        None
    }
}

pub(super) fn is_contextual_this_reference(reference: &Reference) -> bool {
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
) -> Option<&'a Symbol> {
    match contextual_selector_target(snapshot, reference) {
        SelectorContextResolution::Resolved(symbol) => return Some(symbol),
        SelectorContextResolution::Unresolved => return None,
        SelectorContextResolution::NotPresent => {}
    }

    let start_symbol = reference
        .containing_symbol
        .or_else(|| enclosing_symbol_for_span(snapshot, reference.span));
    contextual_symbol_target(snapshot, start_symbol, reference)
}

fn contextual_symbol_target<'a>(
    snapshot: &'a DocumentSnapshot,
    start_symbol: Option<SymbolId>,
    reference: &Reference,
) -> Option<&'a Symbol> {
    let matches_kind: fn(SymbolKind) -> bool = match reference.target_hint {
        strz_analysis::ReferenceTargetHint::Element => is_element_symbol_kind,
        strz_analysis::ReferenceTargetHint::Deployment => is_deployment_symbol_kind,
        strz_analysis::ReferenceTargetHint::Relationship
        | strz_analysis::ReferenceTargetHint::ElementOrRelationship => return None,
    };

    let mut current = start_symbol;
    while let Some(symbol_id) = current {
        let symbol = snapshot.symbols().get(symbol_id.0)?;
        if matches_kind(symbol.kind) {
            return Some(symbol);
        }
        current = symbol.parent;
    }

    None
}

enum SelectorContextResolution<'a> {
    NotPresent,
    Resolved(&'a Symbol),
    Unresolved,
}

fn contextual_selector_target<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
) -> SelectorContextResolution<'a> {
    let Some(selector_target) = snapshot
        .element_directives()
        .iter()
        .filter(|directive| span_within(directive.span, reference.span))
        .min_by_key(|directive| directive.span.end_byte - directive.span.start_byte)
        .map(|directive| &directive.target)
    else {
        return SelectorContextResolution::NotPresent;
    };
    if !matches!(
        selector_target.value_kind,
        DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
    ) {
        return SelectorContextResolution::Unresolved;
    }

    let mut candidates = snapshot
        .symbols()
        .iter()
        .filter(|symbol| symbol.binding_name.as_deref() == Some(&selector_target.normalized_text))
        .filter(|symbol| reference_could_target_symbol_kind(reference, symbol.kind));
    let Some(first) = candidates.next() else {
        return SelectorContextResolution::Unresolved;
    };
    if candidates.next().is_some() {
        return SelectorContextResolution::Unresolved;
    }

    SelectorContextResolution::Resolved(first)
}

fn enclosing_symbol_for_span(
    snapshot: &DocumentSnapshot,
    span: strz_analysis::TextSpan,
) -> Option<SymbolId> {
    snapshot
        .symbols()
        .iter()
        .filter(|symbol| span_within(symbol.span, span))
        .min_by_key(|symbol| symbol.span.end_byte - symbol.span.start_byte)
        .map(|symbol| symbol.id)
}

fn instance_type_symbol_for_reference<'a>(
    snapshot: &'a DocumentSnapshot,
    reference: &Reference,
) -> Option<&'a Symbol> {
    if reference.kind == ReferenceKind::InstanceTarget {
        return resolve_reference(snapshot, reference);
    }

    let symbol = resolve_reference(snapshot, reference)?;
    instance_type_symbol(snapshot, symbol)
}

fn instance_type_symbol<'a>(snapshot: &'a DocumentSnapshot, symbol: &Symbol) -> Option<&'a Symbol> {
    if !is_instance_symbol(symbol) {
        return None;
    }

    let (_, reference) = instance_target_reference(snapshot, symbol.id)?;
    resolve_reference(snapshot, reference)
}

fn instance_target_reference(
    snapshot: &DocumentSnapshot,
    symbol_id: SymbolId,
) -> Option<(usize, &Reference)> {
    let mut matches = snapshot
        .references()
        .iter()
        .enumerate()
        .filter(|(_, reference)| {
            reference.kind == ReferenceKind::InstanceTarget
                && reference.containing_symbol == Some(symbol_id)
        });
    let first = matches.next()?;

    if matches.next().is_some() {
        return None;
    }

    Some(first)
}

const fn is_instance_symbol(symbol: &Symbol) -> bool {
    matches!(
        symbol.kind,
        SymbolKind::ContainerInstance | SymbolKind::SoftwareSystemInstance
    )
}

pub(super) fn symbol_matches_reference(symbol: &Symbol, reference: &Reference) -> bool {
    let Some(binding_name) = symbol.binding_name.as_deref() else {
        return false;
    };

    binding_name == reference.raw_text && reference_could_target_symbol_kind(reference, symbol.kind)
}

const fn span_within(outer: strz_analysis::TextSpan, inner: strz_analysis::TextSpan) -> bool {
    outer.start_byte <= inner.start_byte && inner.end_byte <= outer.end_byte
}

pub(super) const fn is_element_symbol_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Person
            | SymbolKind::SoftwareSystem
            | SymbolKind::Container
            | SymbolKind::Component
    )
}

pub(super) const fn is_deployment_symbol_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::DeploymentEnvironment
            | SymbolKind::DeploymentNode
            | SymbolKind::InfrastructureNode
            | SymbolKind::ContainerInstance
            | SymbolKind::SoftwareSystemInstance
    )
}

pub(super) const fn reference_could_target_symbol_kind(
    reference: &Reference,
    target_kind: SymbolKind,
) -> bool {
    match reference.target_hint {
        strz_analysis::ReferenceTargetHint::Element => is_element_symbol_kind(target_kind),
        strz_analysis::ReferenceTargetHint::Deployment => is_deployment_symbol_kind(target_kind),
        strz_analysis::ReferenceTargetHint::Relationship => {
            matches!(target_kind, SymbolKind::Relationship)
        }
        strz_analysis::ReferenceTargetHint::ElementOrRelationship => true,
    }
}

const fn span_contains(start_byte: usize, end_byte: usize, offset: usize) -> bool {
    if start_byte == end_byte {
        offset == start_byte
    } else {
        start_byte <= offset && offset < end_byte
    }
}
