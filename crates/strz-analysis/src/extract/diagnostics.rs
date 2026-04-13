//! Extraction of syntax diagnostics from `ERROR` and `MISSING` tree-sitter nodes.

use tree_sitter::{Node, Tree};

use crate::diagnostics::RuledDiagnostic;
use crate::span::TextSpan;

pub fn collect(tree: &Tree) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();
    collect_from_node(tree.root_node(), &mut diagnostics);
    diagnostics
}

fn collect_from_node(node: Node<'_>, diagnostics: &mut Vec<RuledDiagnostic>) {
    if node.is_missing() {
        diagnostics.push(RuledDiagnostic::missing_node(
            node.kind(),
            TextSpan::from_node(node),
        ));
    } else if node.is_error() {
        diagnostics.push(RuledDiagnostic::unexpected_syntax(TextSpan::from_node(
            node,
        )));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_from_node(child, diagnostics);
    }
}
