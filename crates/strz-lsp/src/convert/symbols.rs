//! Convert extracted analysis symbols into nested LSP document symbols.

use std::collections::BTreeMap;

use strz_analysis::{DocumentSnapshot, Symbol, SymbolId, SymbolKind};
use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind as LspSymbolKind};

use crate::{convert::positions::span_to_range, documents::DocumentState};

/// Converts analysis symbols into nested LSP document symbols.
#[must_use]
pub fn document_symbols(
    document: &DocumentState,
    snapshot: &DocumentSnapshot,
) -> Vec<DocumentSymbol> {
    let mut children_by_parent: BTreeMap<Option<SymbolId>, Vec<&Symbol>> = BTreeMap::new();

    for symbol in snapshot.symbols() {
        children_by_parent
            .entry(symbol.parent)
            .or_default()
            .push(symbol);
    }

    children_by_parent
        .get(&None)
        .into_iter()
        .flat_map(|symbols| symbols.iter())
        .filter_map(|symbol| build_symbol(document, &children_by_parent, symbol))
        .collect()
}

fn build_symbol(
    document: &DocumentState,
    children_by_parent: &BTreeMap<Option<SymbolId>, Vec<&Symbol>>,
    symbol: &Symbol,
) -> Option<DocumentSymbol> {
    let range = span_to_range(document.line_index(), symbol.span)?;
    let children = children_by_parent
        .get(&Some(symbol.id))
        .map(|symbols| {
            symbols
                .iter()
                .filter_map(|child| build_symbol(document, children_by_parent, child))
                .collect::<Vec<_>>()
        })
        .filter(|symbols| !symbols.is_empty());

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name: symbol.display_name.clone(),
        detail: symbol.binding_name.clone(),
        kind: to_lsp_symbol_kind(symbol.kind),
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children,
    })
}

const fn to_lsp_symbol_kind(kind: SymbolKind) -> LspSymbolKind {
    match kind {
        SymbolKind::Person
        | SymbolKind::SoftwareSystem
        | SymbolKind::Container
        | SymbolKind::Component
        | SymbolKind::DeploymentNode
        | SymbolKind::InfrastructureNode
        | SymbolKind::ContainerInstance
        | SymbolKind::SoftwareSystemInstance => LspSymbolKind::OBJECT,
        SymbolKind::Relationship => LspSymbolKind::EVENT,
    }
}
