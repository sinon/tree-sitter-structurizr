//! Extraction of syntax diagnostics from `ERROR` and `MISSING` nodes.

use tree_sitter::{Node, Tree};

use crate::diagnostics::SyntaxDiagnostic;
use crate::span::TextSpan;

pub fn collect(tree: &Tree) -> Vec<SyntaxDiagnostic> {
    let mut diagnostics = Vec::new();
    collect_from_node(tree.root_node(), &mut diagnostics);
    diagnostics
}

fn collect_from_node(node: Node<'_>, diagnostics: &mut Vec<SyntaxDiagnostic>) {
    if node.is_missing() {
        diagnostics.push(SyntaxDiagnostic::missing_node(
            node.kind(),
            TextSpan::from_node(node),
        ));
    } else if node.is_error() {
        diagnostics.push(SyntaxDiagnostic::unexpected_syntax(TextSpan::from_node(
            node,
        )));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_from_node(child, diagnostics);
    }
}
