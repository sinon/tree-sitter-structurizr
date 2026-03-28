//! Extraction of ordered string-constant facts from the current document tree.

use tree_sitter::{Node, Tree};

use crate::{
    constants::ConstantDefinition,
    includes::{DirectiveContainer, normalized_directive_value},
    span::TextSpan,
};

pub fn collect(tree: &Tree, source: &str) -> Vec<ConstantDefinition> {
    let mut constants = Vec::new();
    collect_from_node(tree.root_node(), source, &mut constants);
    constants
}

fn collect_from_node(node: Node<'_>, source: &str, constants: &mut Vec<ConstantDefinition>) {
    if matches!(node.kind(), "const_directive" | "constant_directive")
        && let (Some(name_node), Some(value_node)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        )
    {
        let raw_name = node_text(name_node, source);
        let raw_value = node_text(value_node, source);

        constants.push(ConstantDefinition {
            name: raw_name,
            value: normalized_directive_value(&raw_value, &crate::DirectiveValueKind::from_node_kind(value_node.kind())),
            span: TextSpan::from_node(node),
            name_span: TextSpan::from_node(name_node),
            value_span: TextSpan::from_node(value_node),
            container: directive_container(node),
        });
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index.try_into().expect("child index should fit in u32")) {
            collect_from_node(child, source, constants);
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
