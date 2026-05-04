//! Hover handler for source-derived identifier summaries.

use std::path::Path;

use strz_analysis::{
    DocumentSnapshot, ReferenceHandle, ReferenceKind, Symbol, SymbolHandle, SymbolKind,
    WorkspaceFacts, WorkspaceInstanceId,
};
use tower_lsp_server::ls_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use tracing::{debug, info};

use crate::{convert::positions::position_to_byte_offset, server::Backend, state::ServerState};

use super::navigation::ResolvedSymbolOrigin;

/// Handles `textDocument/hover` for the current bounded navigation slice.
///
/// # Errors
///
/// This handler currently does not emit JSON-RPC errors. Unsupported or
/// unresolved hover sites are reported as `Ok(None)`.
pub async fn hover(
    backend: &Backend,
    params: HoverParams,
) -> tower_lsp_server::jsonrpc::Result<Option<Hover>> {
    let uri = params
        .text_document_position_params
        .text_document
        .uri
        .clone();
    let position = params.text_document_position_params.position;
    let hover = {
        let state = backend.state().read().await;
        let Some(document) = state.documents().get(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "hover skipped because the document is not open"
            );
            return Ok(None);
        };
        let Some(snapshot) = state.snapshot(&uri) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "hover skipped because no snapshot is cached"
            );
            return Ok(None);
        };
        let Some(offset) = position_to_byte_offset(document.line_index(), position) else {
            debug!(
                uri = uri.as_str(),
                ?position,
                "hover skipped because the position was invalid"
            );
            return Ok(None);
        };

        debug!(uri = uri.as_str(), ?position, offset, "running hover");
        let Some(target) =
            super::navigation::resolved_symbol_target_at_offset(&state, document, snapshot, offset)
        else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "hover returned no result"
            );
            return Ok(None);
        };
        let context = hover_context(&state, &target);
        let value = render_hover_markdown(target.symbol, &context);
        drop(state);

        Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        }
    };

    info!(uri = uri.as_str(), ?position, "hover resolved a symbol");
    Ok(Some(hover))
}

#[derive(Debug, Default)]
struct HoverContext {
    canonical_key: Option<String>,
    parent_chain: Option<String>,
    declaration_path: Option<String>,
    relationship_endpoints: Option<RelationshipEndpointContext>,
}

#[derive(Debug)]
struct RelationshipEndpointContext {
    source: String,
    destination: String,
}

fn hover_context(
    state: &ServerState,
    target: &super::navigation::ResolvedSymbolTarget<'_>,
) -> HoverContext {
    if !origin_allows_richer_context(target.origin) {
        return HoverContext::default();
    }

    let Some(workspace_facts) = state.workspace_facts() else {
        return HoverContext::default();
    };
    let Some(handle) = target.handle.as_ref() else {
        return HoverContext::default();
    };
    if target.candidate_instances.is_empty() {
        return HoverContext::default();
    }

    HoverContext {
        canonical_key: unanimous_binding_key(workspace_facts, &target.candidate_instances, handle),
        parent_chain: parent_chain(target.snapshot, target.symbol),
        declaration_path: declaration_path(state, target.snapshot),
        relationship_endpoints: relationship_endpoint_context(workspace_facts, target),
    }
}

const fn origin_allows_richer_context(origin: ResolvedSymbolOrigin) -> bool {
    match origin {
        ResolvedSymbolOrigin::Reference {
            kind: ReferenceKind::ElementSelectorTarget,
            ..
        }
        | ResolvedSymbolOrigin::Reference {
            is_contextual_this: true,
            ..
        } => false,
        ResolvedSymbolOrigin::Declaration | ResolvedSymbolOrigin::Reference { .. } => true,
    }
}

fn render_hover_markdown(symbol: &Symbol, context: &HoverContext) -> String {
    let binding_name = symbol
        .binding_name
        .as_deref()
        .expect("hovered symbols should be bindable");
    let mut sections = Vec::new();
    let mut heading_lines = vec![format!(
        "**{}** `{binding_name}`",
        symbol_kind_label(symbol.kind)
    )];

    if symbol.display_name != binding_name {
        heading_lines.push(symbol.display_name.clone());
    }
    sections.push(heading_lines.join("\n"));

    if let Some(description) = symbol.description.as_deref() {
        sections.push(description.to_owned());
    }

    let mut metadata_lines = Vec::new();
    if let Some(technology) = symbol.technology.as_deref() {
        metadata_lines.push(format!("**Technology:** {technology}"));
    }
    if !symbol.tags.is_empty() {
        metadata_lines.push(format!("**Tags:** {}", symbol.tags.join(", ")));
    }
    if let Some(url) = symbol.url.as_deref() {
        metadata_lines.push(format!("**URL:** <{url}>"));
    }
    if !metadata_lines.is_empty() {
        sections.push(metadata_lines.join("  \n"));
    }

    if let Some(context_section) = render_context_section(context) {
        sections.push(context_section);
    }

    sections.join("\n\n")
}

fn render_context_section(context: &HoverContext) -> Option<String> {
    let mut lines = Vec::new();

    if let Some(canonical_key) = context.canonical_key.as_deref() {
        lines.push(format!("**Canonical key:** `{canonical_key}`"));
    }
    if let Some(parent_chain) = context.parent_chain.as_deref() {
        lines.push(format!("**Parent chain:** {parent_chain}"));
    }
    if let Some(declaration_path) = context.declaration_path.as_deref() {
        lines.push(format!("**Declaration path:** `{declaration_path}`"));
    }
    if let Some(endpoints) = context.relationship_endpoints.as_ref() {
        lines.push(format!(
            "**Endpoints:** {} → {}",
            endpoints.source, endpoints.destination
        ));
    }

    (!lines.is_empty()).then(|| lines.join("  \n"))
}

fn unanimous_binding_key(
    workspace_facts: &WorkspaceFacts,
    candidate_instances: &[WorkspaceInstanceId],
    handle: &SymbolHandle,
) -> Option<String> {
    let mut binding_key = None::<String>;

    for instance_id in candidate_instances {
        let current = workspace_facts
            .workspace_index(*instance_id)?
            .unique_binding_key_for_symbol(handle)?;
        if binding_key
            .as_deref()
            .is_some_and(|existing| existing != current)
        {
            return None;
        }

        binding_key = Some(current.to_owned());
    }

    binding_key
}

fn parent_chain(snapshot: &DocumentSnapshot, symbol: &Symbol) -> Option<String> {
    let mut chain = Vec::new();
    let mut parent = symbol.parent;

    while let Some(parent_id) = parent {
        let parent_symbol = snapshot.symbols().get(parent_id.0)?;
        let binding_name = parent_symbol.binding_name.as_deref()?;
        chain.push(format!(
            "{} `{binding_name}`",
            symbol_kind_label(parent_symbol.kind)
        ));
        parent = parent_symbol.parent;
    }

    if chain.is_empty() {
        return None;
    }

    chain.reverse();
    Some(chain.join(" → "))
}

fn declaration_path(state: &ServerState, snapshot: &DocumentSnapshot) -> Option<String> {
    let declaration_path = snapshot.location()?.path();
    let mut matches = state
        .workspace_roots()
        .iter()
        .filter_map(tower_lsp_server::ls_types::Uri::to_file_path)
        .filter_map(|root| display_path_relative_to_root(declaration_path, root.as_ref()))
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();

    match matches.as_slice() {
        [path] => Some(path.clone()),
        _ => None,
    }
}

fn display_path_relative_to_root(path: &Path, root: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    (!relative.as_os_str().is_empty()).then(|| display_path(relative))
}

fn display_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn relationship_endpoint_context(
    workspace_facts: &WorkspaceFacts,
    target: &super::navigation::ResolvedSymbolTarget<'_>,
) -> Option<RelationshipEndpointContext> {
    let (source_reference, destination_reference) = relationship_endpoint_references(target)?;
    let source_handle = super::navigation::unanimous_resolved_symbol(
        workspace_facts,
        &target.candidate_instances,
        &source_reference,
    )?;
    let destination_handle = super::navigation::unanimous_resolved_symbol(
        workspace_facts,
        &target.candidate_instances,
        &destination_reference,
    )?;

    Some(RelationshipEndpointContext {
        source: endpoint_label(workspace_facts, &target.candidate_instances, &source_handle)?,
        destination: endpoint_label(
            workspace_facts,
            &target.candidate_instances,
            &destination_handle,
        )?,
    })
}

fn relationship_endpoint_references(
    target: &super::navigation::ResolvedSymbolTarget<'_>,
) -> Option<(ReferenceHandle, ReferenceHandle)> {
    if target.symbol.kind != SymbolKind::Relationship {
        return None;
    }

    let document_id = target.handle.as_ref()?.document().clone();
    let mut source = None;
    let mut destination = None;

    for (index, reference) in target.snapshot.references().iter().enumerate() {
        if reference.containing_symbol != Some(target.symbol.id) {
            continue;
        }
        if reference.raw_text == "this" {
            return None;
        }

        match reference.kind {
            ReferenceKind::RelationshipSource | ReferenceKind::DeploymentRelationshipSource => {
                set_once(
                    &mut source,
                    ReferenceHandle::new(document_id.clone(), index),
                )?;
            }
            ReferenceKind::RelationshipDestination
            | ReferenceKind::DeploymentRelationshipDestination => {
                set_once(
                    &mut destination,
                    ReferenceHandle::new(document_id.clone(), index),
                )?;
            }
            ReferenceKind::ElementSelectorTarget
            | ReferenceKind::InstanceTarget
            | ReferenceKind::ViewScope
            | ReferenceKind::ViewInclude
            | ReferenceKind::ViewExclude
            | ReferenceKind::ViewAnimation => {}
        }
    }

    Some((source?, destination?))
}

fn endpoint_label(
    workspace_facts: &WorkspaceFacts,
    candidate_instances: &[WorkspaceInstanceId],
    handle: &SymbolHandle,
) -> Option<String> {
    let snapshot = workspace_facts.document(handle.document())?.snapshot();
    let symbol = snapshot.symbols().get(handle.symbol_id().0)?;
    let binding_key = unanimous_binding_key(workspace_facts, candidate_instances, handle)?;

    Some(format!(
        "{} `{binding_key}`",
        symbol_kind_label(symbol.kind)
    ))
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Option<()> {
    if slot.replace(value).is_some() {
        return None;
    }

    Some(())
}

const fn symbol_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Person => "Person",
        SymbolKind::SoftwareSystem => "Software System",
        SymbolKind::Container => "Container",
        SymbolKind::Component => "Component",
        SymbolKind::DeploymentEnvironment => "Deployment Environment",
        SymbolKind::DeploymentNode => "Deployment Node",
        SymbolKind::InfrastructureNode => "Infrastructure Node",
        SymbolKind::ContainerInstance => "Container Instance",
        SymbolKind::SoftwareSystemInstance => "Software System Instance",
        SymbolKind::Relationship => "Relationship",
    }
}
