//! Extraction of raw `!include` directive facts from the current document tree.

use tree_sitter::{Node, Tree};

use crate::includes::{DirectiveContainer, DirectiveValueKind, IncludeDirective};
use crate::span::TextSpan;

pub fn collect(tree: &Tree, source: &str) -> Vec<IncludeDirective> {
    let mut directives = Vec::new();
    collect_from_node(tree.root_node(), source, &mut directives);
    directives
}

fn collect_from_node(node: Node<'_>, source: &str, directives: &mut Vec<IncludeDirective>) {
    if node.kind() == "include_directive"
        && let Some(value_node) = node.child_by_field_name("value") {
            directives.push(IncludeDirective {
                raw_value: node_text(value_node, source),
                value_kind: DirectiveValueKind::from_node_kind(value_node.kind()),
                span: TextSpan::from_node(node),
                value_span: TextSpan::from_node(value_node),
                container: directive_container(node),
            });
        }
    // TODO: Can we rework this to use an iterator and avoid the expect (that should always succeed)
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index.try_into().expect("child index should fit in u32")) {
            collect_from_node(child, source, directives);
        }
    }
}

fn directive_container(node: Node<'_>) -> DirectiveContainer {
    let mut current = node;

    while let Some(parent) = current.parent() {
        match parent.kind() {
            "source_file" | "workspace_block" | "model_block" => {
                return DirectiveContainer::from_enclosing_kind(parent.kind());
            }
            _ => current = parent,
        }
    }

    DirectiveContainer::SourceFile
}

fn node_text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .expect("node text should be utf-8")
        .to_owned()
}
