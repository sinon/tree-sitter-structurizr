//! Rename handlers for the first bounded identifier-editing slice.

use std::{collections::HashMap, path::Path};

use line_index::LineIndex;
use structurizr_analysis::{
    DirectiveContainer, DocumentId, DocumentSnapshot, ElementIdentifierMode, IdentifierMode,
    Reference, ReferenceHandle, ReferenceResolutionStatus, Symbol, SymbolHandle, SymbolKind,
    TextSpan, WorkspaceFacts, WorkspaceIndex, WorkspaceInstanceId,
};
use tower_lsp_server::jsonrpc::Error;
use tower_lsp_server::ls_types::{
    PrepareRenameResponse, RenameParams, TextDocumentPositionParams, TextEdit, Uri, WorkspaceEdit,
};
use tracing::{debug, info};

use crate::{
    convert::{
        positions::{position_to_byte_offset, span_to_range},
        uris::file_uri_from_path,
    },
    documents::DocumentState,
    server::Backend,
    state::ServerState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenameEditSite {
    uri: Uri,
    document_id: Option<DocumentId>,
    span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenamePlan {
    current_name: String,
    request_span: TextSpan,
    edit_sites: Vec<RenameEditSite>,
}

/// Handles `textDocument/prepareRename` for the bounded flat-identifier slice.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Unsupported or
/// unresolved rename sites are reported as `Ok(None)`.
pub async fn prepare_rename(
    backend: &Backend,
    params: TextDocumentPositionParams,
) -> tower_lsp_server::jsonrpc::Result<Option<PrepareRenameResponse>> {
    let uri = params.text_document.uri.clone();
    let position = params.position;
    let response = {
        let state = backend.state().read().await;
        let Some(document) = state.documents().get(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "prepareRename skipped because the document is not open"
            );
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "prepareRename skipped because no snapshot is cached"
            );
            return Ok(None);
        };
        let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "prepareRename skipped because the position was invalid"
            );
            return Ok(None);
        };
        let Some(plan) = rename_plan(&state, document, snapshot, offset) else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "prepareRename returned no result"
            );
            return Ok(None);
        };
        let Some(range) = span_to_range(document.line_index(), plan.request_span) else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "prepareRename could not convert the request span"
            );
            return Ok(None);
        };
        drop(state);

        PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: plan.current_name,
        }
    };

    info!(
        uri = uri.as_str(),
        ?position,
        "prepareRename resolved a rename target"
    );
    Ok(Some(response))
}

/// Handles `textDocument/rename` for the bounded flat-identifier slice.
///
/// # Errors
///
/// Returns `invalid_params` when the requested new name is not a valid
/// identifier-shaped token for the current grammar.
pub async fn rename(
    backend: &Backend,
    params: RenameParams,
) -> tower_lsp_server::jsonrpc::Result<Option<WorkspaceEdit>> {
    let uri = params.text_document_position.text_document.uri.clone();
    let position = params.text_document_position.position;
    let new_name = params.new_name;
    if !is_valid_identifier(&new_name) {
        return Err(Error::invalid_params(
            "rename newName must match the Structurizr identifier token shape",
        ));
    }

    let edit = {
        let state = backend.state().read().await;
        let Some(document) = state.documents().get(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "rename skipped because the document is not open"
            );
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "rename skipped because no snapshot is cached"
            );
            return Ok(None);
        };
        let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "rename skipped because the position was invalid"
            );
            return Ok(None);
        };
        let Some(plan) = rename_plan(&state, document, snapshot, offset) else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "rename returned no result"
            );
            return Ok(None);
        };
        let Some(edit) = workspace_edit_for_plan(&state, &plan, &new_name) else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "rename could not materialize a workspace edit"
            );
            return Ok(None);
        };
        drop(state);
        edit
    };

    info!(
        uri = uri.as_str(),
        ?position,
        "rename resolved a workspace edit"
    );
    Ok(Some(edit))
}

fn rename_plan(
    state: &ServerState,
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    offset: usize,
) -> Option<RenamePlan> {
    if let Some(document_id) = document.workspace_document_id()
        && let Some(workspace_facts) = state.workspace_facts()
    {
        let candidate_instances = workspace_facts
            .candidate_instances_for(document_id)
            .copied()
            .collect::<Vec<_>>();
        if !candidate_instances.is_empty() {
            return workspace_rename_plan(
                workspace_facts,
                snapshot,
                offset,
                document_id,
                &candidate_instances,
            );
        }
    }

    same_document_rename_plan(document, snapshot, offset)
}

fn workspace_rename_plan(
    workspace_facts: &WorkspaceFacts,
    snapshot: &DocumentSnapshot,
    offset: usize,
    current_document_id: &DocumentId,
    candidate_instances: &[WorkspaceInstanceId],
) -> Option<RenamePlan> {
    let site = super::navigation::navigation_site_at_offset(snapshot, offset)?;
    let request_span = match site {
        super::navigation::NavigationSite::Symbol(symbol) => symbol.binding_span?,
        super::navigation::NavigationSite::Reference { reference, .. } => reference.span,
    };

    let target_handle = match site {
        super::navigation::NavigationSite::Symbol(symbol) => {
            SymbolHandle::new(current_document_id.clone(), symbol.id)
        }
        super::navigation::NavigationSite::Reference { index, .. } => {
            let reference_handle = ReferenceHandle::new(current_document_id.clone(), index);
            super::navigation::unanimous_resolved_symbol(
                workspace_facts,
                candidate_instances,
                &reference_handle,
            )?
        }
    };
    let target_symbol = workspace_symbol(workspace_facts, &target_handle)?;
    if !is_supported_rename_symbol_kind(target_symbol.kind) {
        return None;
    }
    let current_name = target_symbol.binding_name.clone()?;

    let mut edit_sites = None;
    for instance_id in candidate_instances {
        let workspace_index = workspace_facts.workspace_index(*instance_id)?;
        let current = workspace_instance_rename_sites(
            workspace_facts,
            workspace_index,
            &target_handle,
            target_symbol,
            &current_name,
        )?;
        if edit_sites
            .as_ref()
            .is_some_and(|existing| existing != &current)
        {
            return None;
        }
        edit_sites = Some(current);
    }

    Some(RenamePlan {
        current_name,
        request_span,
        edit_sites: edit_sites?,
    })
}

fn workspace_instance_rename_sites(
    workspace_facts: &WorkspaceFacts,
    workspace_index: &WorkspaceIndex,
    target_handle: &SymbolHandle,
    target_symbol: &Symbol,
    current_name: &str,
) -> Option<Vec<RenameEditSite>> {
    if !target_is_renameable_in_workspace(
        workspace_index,
        target_handle,
        target_symbol,
        current_name,
    ) {
        return None;
    }
    if workspace_instance_has_ambiguous_same_text_references(
        workspace_facts,
        workspace_index,
        target_handle,
        current_name,
        target_symbol.kind,
    ) {
        return None;
    }

    let target_snapshot = workspace_facts
        .document(target_handle.document())?
        .snapshot();
    let declaration = target_snapshot.symbols().get(target_handle.symbol_id().0)?;
    let mut sites = vec![workspace_edit_site(
        target_handle.document(),
        declaration.binding_span?,
    )?];

    let mut reference_handles = workspace_index
        .references_for_symbol(target_handle)
        .cloned()
        .collect::<Vec<_>>();
    reference_handles.sort();
    reference_handles.dedup();

    for handle in reference_handles {
        let snapshot = workspace_facts.document(handle.document())?.snapshot();
        let reference = snapshot.references().get(handle.reference_index())?;
        sites.push(workspace_edit_site(handle.document(), reference.span)?);
    }

    sort_and_dedup_sites(&mut sites);
    Some(sites)
}

fn same_document_rename_plan(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
    offset: usize,
) -> Option<RenamePlan> {
    let site = super::navigation::navigation_site_at_offset(snapshot, offset)?;
    let request_span = match site {
        super::navigation::NavigationSite::Symbol(symbol) => symbol.binding_span?,
        super::navigation::NavigationSite::Reference { reference, .. } => reference.span,
    };

    let target_symbol = match site {
        super::navigation::NavigationSite::Symbol(symbol) => symbol,
        super::navigation::NavigationSite::Reference { reference, .. } => {
            super::navigation::resolve_reference(snapshot, reference)?
        }
    };
    if !is_supported_rename_symbol_kind(target_symbol.kind) {
        return None;
    }
    if is_element_symbol_kind(target_symbol.kind) && !snapshot_uses_flat_identifier_mode(snapshot) {
        return None;
    }

    let current_name = target_symbol.binding_name.clone()?;
    if !target_is_locally_unique(snapshot, target_symbol) {
        return None;
    }
    if same_document_has_ambiguous_same_text_references(snapshot, target_symbol, &current_name) {
        return None;
    }

    let mut edit_sites = vec![RenameEditSite {
        uri: document.uri().clone(),
        document_id: document.workspace_document_id().cloned(),
        span: target_symbol.binding_span?,
    }];
    edit_sites.extend(
        snapshot
            .references()
            .iter()
            .filter(|reference| {
                super::navigation::resolve_reference(snapshot, reference) == Some(target_symbol)
            })
            .map(|reference| RenameEditSite {
                uri: document.uri().clone(),
                document_id: document.workspace_document_id().cloned(),
                span: reference.span,
            }),
    );
    sort_and_dedup_sites(&mut edit_sites);

    Some(RenamePlan {
        current_name,
        request_span,
        edit_sites,
    })
}

fn workspace_edit_for_plan(
    state: &ServerState,
    plan: &RenamePlan,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    let mut changes = HashMap::<Uri, Vec<TextEdit>>::new();

    for site in &plan.edit_sites {
        let (uri, range) = if let Some(document_id) = site.document_id.as_ref()
            && let Some(document) = open_document_by_id(state, document_id)
        {
            (
                document.uri().clone(),
                span_to_range(document.line_index(), site.span)?,
            )
        } else if let Some(document) = state.documents().get(&site.uri) {
            (
                document.uri().clone(),
                span_to_range(document.line_index(), site.span)?,
            )
        } else {
            let document_id = site.document_id.as_ref()?;
            let snapshot = state.workspace_facts()?.document(document_id)?.snapshot();
            let line_index = LineIndex::new(snapshot.source());
            (site.uri.clone(), span_to_range(&line_index, site.span)?)
        };

        changes.entry(uri).or_default().push(TextEdit {
            range,
            new_text: new_name.to_owned(),
        });
    }

    Some(WorkspaceEdit::new(changes))
}

fn workspace_instance_has_ambiguous_same_text_references(
    workspace_facts: &WorkspaceFacts,
    workspace_index: &WorkspaceIndex,
    target_handle: &SymbolHandle,
    current_name: &str,
    target_kind: SymbolKind,
) -> bool {
    for document_id in workspace_index.documents() {
        let Some(snapshot) = workspace_facts
            .document(document_id)
            .map(structurizr_analysis::WorkspaceDocument::snapshot)
        else {
            return true;
        };

        for (reference_index, reference) in snapshot.references().iter().enumerate() {
            if reference.raw_text != current_name
                || !reference_could_target_symbol(reference, target_kind)
            {
                continue;
            }

            let handle = ReferenceHandle::new(document_id.clone(), reference_index);
            let Some(status) = workspace_index.reference_resolution(&handle) else {
                return true;
            };
            match status {
                ReferenceResolutionStatus::Resolved(resolved) if resolved == target_handle => {}
                ReferenceResolutionStatus::Resolved(_)
                | ReferenceResolutionStatus::UnresolvedNoMatch => {}
                ReferenceResolutionStatus::AmbiguousDuplicateBinding
                | ReferenceResolutionStatus::AmbiguousElementVsRelationship
                | ReferenceResolutionStatus::DeferredByScopePolicy => return true,
            }
        }
    }

    false
}

fn same_document_has_ambiguous_same_text_references(
    snapshot: &DocumentSnapshot,
    target_symbol: &Symbol,
    current_name: &str,
) -> bool {
    snapshot.references().iter().any(|reference| {
        reference.raw_text == current_name
            && reference_could_target_symbol(reference, target_symbol.kind)
            && super::navigation::resolve_reference(snapshot, reference).is_none()
    })
}

fn target_is_renameable_in_workspace(
    workspace_index: &WorkspaceIndex,
    target_handle: &SymbolHandle,
    target_symbol: &Symbol,
    current_name: &str,
) -> bool {
    match target_symbol.kind {
        SymbolKind::Person
        | SymbolKind::SoftwareSystem
        | SymbolKind::Container
        | SymbolKind::Component => {
            if workspace_index.element_identifier_mode_for(target_handle.document())
                != Some(ElementIdentifierMode::Flat)
            {
                return false;
            }

            workspace_index
                .unique_element_bindings()
                .get(current_name)
                .is_some_and(|handle| handle == target_handle)
                && !workspace_index
                    .duplicate_element_bindings()
                    .contains_key(current_name)
        }
        SymbolKind::DeploymentNode
        | SymbolKind::InfrastructureNode
        | SymbolKind::ContainerInstance
        | SymbolKind::SoftwareSystemInstance => {
            workspace_index
                .unique_deployment_bindings()
                .get(current_name)
                .is_some_and(|handle| handle == target_handle)
                && !workspace_index
                    .duplicate_deployment_bindings()
                    .contains_key(current_name)
        }
        SymbolKind::Relationship => false,
    }
}

fn target_is_locally_unique(snapshot: &DocumentSnapshot, target_symbol: &Symbol) -> bool {
    let Some(current_name) = target_symbol.binding_name.as_deref() else {
        return false;
    };

    let matching_symbols = snapshot
        .symbols()
        .iter()
        .filter(|symbol| symbol.binding_name.as_deref() == Some(current_name))
        .filter(|symbol| symbol_in_same_binding_family(symbol.kind, target_symbol.kind))
        .count();

    matching_symbols == 1
}

fn workspace_symbol<'a>(
    workspace_facts: &'a WorkspaceFacts,
    handle: &SymbolHandle,
) -> Option<&'a Symbol> {
    workspace_facts
        .document(handle.document())?
        .snapshot()
        .symbols()
        .get(handle.symbol_id().0)
}

fn workspace_edit_site(document_id: &DocumentId, span: TextSpan) -> Option<RenameEditSite> {
    Some(RenameEditSite {
        uri: file_uri_from_path(Path::new(document_id.as_str()))?,
        document_id: Some(document_id.clone()),
        span,
    })
}

fn open_document_by_id<'a>(
    state: &'a ServerState,
    document_id: &DocumentId,
) -> Option<&'a DocumentState> {
    state.documents().iter().find(|document| {
        document
            .workspace_document_id()
            .is_some_and(|candidate| candidate == document_id)
    })
}

fn sort_and_dedup_sites(sites: &mut Vec<RenameEditSite>) {
    sites.sort_by(|left, right| {
        left.uri
            .as_str()
            .cmp(right.uri.as_str())
            .then_with(|| left.span.start_byte.cmp(&right.span.start_byte))
            .then_with(|| left.span.end_byte.cmp(&right.span.end_byte))
    });
    sites.dedup_by(|left, right| left.uri == right.uri && left.span == right.span);
}

const fn is_supported_rename_symbol_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Person
            | SymbolKind::SoftwareSystem
            | SymbolKind::Container
            | SymbolKind::Component
            | SymbolKind::DeploymentNode
            | SymbolKind::InfrastructureNode
            | SymbolKind::ContainerInstance
            | SymbolKind::SoftwareSystemInstance
    )
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

const fn symbol_in_same_binding_family(left: SymbolKind, right: SymbolKind) -> bool {
    (is_element_symbol_kind(left) && is_element_symbol_kind(right))
        || (is_deployment_symbol_kind(left) && is_deployment_symbol_kind(right))
}

const fn is_deployment_symbol_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::DeploymentNode
            | SymbolKind::InfrastructureNode
            | SymbolKind::ContainerInstance
            | SymbolKind::SoftwareSystemInstance
    )
}

const fn reference_could_target_symbol(reference: &Reference, target_kind: SymbolKind) -> bool {
    if is_element_symbol_kind(target_kind) {
        return matches!(
            reference.target_hint,
            structurizr_analysis::ReferenceTargetHint::Element
                | structurizr_analysis::ReferenceTargetHint::ElementOrRelationship
        );
    }

    if is_deployment_symbol_kind(target_kind) {
        return matches!(
            reference.target_hint,
            structurizr_analysis::ReferenceTargetHint::Deployment
        );
    }

    false
}

fn snapshot_uses_flat_identifier_mode(snapshot: &DocumentSnapshot) -> bool {
    matches!(
        snapshot_model_identifier_mode(snapshot)
            .or_else(|| snapshot_workspace_identifier_mode(snapshot)),
        Some(IdentifierMode::Flat) | None
    )
}

fn snapshot_model_identifier_mode(snapshot: &DocumentSnapshot) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(snapshot, &DirectiveContainer::Model)
}

fn snapshot_workspace_identifier_mode(snapshot: &DocumentSnapshot) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(snapshot, &DirectiveContainer::Workspace)
}

fn last_identifier_mode_for_container(
    snapshot: &DocumentSnapshot,
    container: &DirectiveContainer,
) -> Option<IdentifierMode> {
    snapshot
        .identifier_modes()
        .iter()
        .rev()
        .find(|fact| fact.container == *container)
        .map(|fact| fact.mode.clone())
}

fn is_valid_identifier(value: &str) -> bool {
    let mut characters = value.bytes();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    characters.all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, b'_' | b'.' | b'-')
    })
}
