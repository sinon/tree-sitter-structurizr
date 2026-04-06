//! Hover handler for source-derived identifier summaries.

use strz_analysis::{Symbol, SymbolKind};
use tower_lsp_server::ls_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use tracing::{debug, info};

use crate::{convert::positions::position_to_byte_offset, server::Backend};

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
        let Some(symbol) =
            super::navigation::resolved_symbol_at_offset(&state, document, snapshot, offset)
        else {
            info!(
                uri = uri.as_str(),
                ?position,
                offset,
                "hover returned no result"
            );
            return Ok(None);
        };
        let value = render_hover_markdown(symbol);
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

fn render_hover_markdown(symbol: &Symbol) -> String {
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

    sections.join("\n\n")
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
