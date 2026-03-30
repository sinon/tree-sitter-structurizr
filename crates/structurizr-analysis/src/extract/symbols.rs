//! Handwritten extraction for bounded-MVP identifier modes, symbols, and references.

use tree_sitter::{Node, Tree};

use crate::includes::{DirectiveContainer, DirectiveValueKind};
use crate::span::TextSpan;
use crate::symbols::{
    IdentifierMode, IdentifierModeFact, Reference, ReferenceKind, ReferenceTargetHint, Symbol,
    SymbolId, SymbolKind,
};

pub fn collect_identifier_modes(tree: &Tree, source: &str) -> Vec<IdentifierModeFact> {
    let mut facts = Vec::new();
    collect_identifier_mode_from_node(tree.root_node(), source, &mut facts);
    facts
}

pub fn collect_symbols_and_references(tree: &Tree, source: &str) -> (Vec<Symbol>, Vec<Reference>) {
    // Keep symbol and reference extraction in one pass so snapshots see a
    // consistent view of declaration hierarchy and cross-reference sites.
    let mut extractor = SymbolExtractor::new(source);
    extractor.visit(tree.root_node(), None);
    (extractor.symbols, extractor.references)
}

fn collect_identifier_mode_from_node(
    node: Node<'_>,
    source: &str,
    facts: &mut Vec<IdentifierModeFact>,
) {
    if node.kind() == "identifiers_directive"
        && let Some(value_node) = node.child_by_field_name("value")
    {
        let raw_value = node_text(value_node, source);
        facts.push(IdentifierModeFact {
            mode: IdentifierMode::from_raw(&normalized_text(value_node, source)),
            raw_value,
            value_kind: DirectiveValueKind::from_node_kind(value_node.kind()),
            span: TextSpan::from_node(node),
            value_span: TextSpan::from_node(value_node),
            container: directive_container(node),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_identifier_mode_from_node(child, source, facts);
    }
}

struct SymbolExtractor<'a> {
    source: &'a str,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
}

impl<'a> SymbolExtractor<'a> {
    const fn new(source: &'a str) -> Self {
        Self {
            source,
            symbols: Vec::new(),
            references: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node<'_>, parent_symbol: Option<SymbolId>) {
        // Declaration nodes build the hierarchical symbol tree, while
        // relationships and views contribute reference edges into that tree.
        match node.kind() {
            "person" => {
                let symbol_id =
                    self.push_declaration_symbol(node, SymbolKind::Person, parent_symbol);
                self.visit_children(node, Some(symbol_id));
            }
            "software_system" => {
                let symbol_id =
                    self.push_declaration_symbol(node, SymbolKind::SoftwareSystem, parent_symbol);
                self.visit_children(node, Some(symbol_id));
            }
            "container" => {
                let symbol_id =
                    self.push_declaration_symbol(node, SymbolKind::Container, parent_symbol);
                self.visit_children(node, Some(symbol_id));
            }
            "component" => {
                let symbol_id =
                    self.push_declaration_symbol(node, SymbolKind::Component, parent_symbol);
                self.visit_children(node, Some(symbol_id));
            }
            "relationship" => {
                let relationship_symbol = self.push_relationship_symbol(node, parent_symbol);
                let containing_symbol = relationship_symbol.or(parent_symbol);
                let (source_kind, destination_kind, target_hint) =
                    relationship_reference_surface(node);
                self.push_relationship_reference(
                    node,
                    "source",
                    source_kind,
                    target_hint,
                    containing_symbol,
                );
                self.push_relationship_reference(
                    node,
                    "destination",
                    destination_kind,
                    target_hint,
                    containing_symbol,
                );
            }
            "deployment_node" => {
                let symbol_id = self.push_named_deployment_symbol(
                    node,
                    SymbolKind::DeploymentNode,
                    parent_symbol,
                );
                self.visit_children(node, symbol_id.or(parent_symbol));
            }
            "infrastructure_node" => {
                let symbol_id = self.push_named_deployment_symbol(
                    node,
                    SymbolKind::InfrastructureNode,
                    parent_symbol,
                );
                self.visit_children(node, symbol_id.or(parent_symbol));
            }
            "container_instance" => {
                let symbol_id =
                    self.push_instance_symbol(node, SymbolKind::ContainerInstance, parent_symbol);
                self.push_instance_target_reference(node, symbol_id.or(parent_symbol));
                self.visit_children(node, symbol_id.or(parent_symbol));
            }
            "software_system_instance" => {
                let symbol_id = self.push_instance_symbol(
                    node,
                    SymbolKind::SoftwareSystemInstance,
                    parent_symbol,
                );
                self.push_instance_target_reference(node, symbol_id.or(parent_symbol));
                self.visit_children(node, symbol_id.or(parent_symbol));
            }
            "system_landscape_view" => self.extract_view(
                node,
                None,
                Some(ReferenceTargetHint::ElementOrRelationship),
                Some(ReferenceTargetHint::Element),
                parent_symbol,
            ),
            "system_context_view" | "container_view" | "component_view" => {
                self.extract_view(
                    node,
                    Some(ReferenceTargetHint::Element),
                    Some(ReferenceTargetHint::ElementOrRelationship),
                    Some(ReferenceTargetHint::Element),
                    parent_symbol,
                );
            }
            // Deployment-view scope still points at the model element, but both
            // `include` and `animation` identifiers refer to deployment-layer
            // bindings such as deployment nodes and instances.
            "deployment_view" => self.extract_view(
                node,
                Some(ReferenceTargetHint::Element),
                Some(ReferenceTargetHint::Deployment),
                Some(ReferenceTargetHint::Deployment),
                parent_symbol,
            ),
            _ => self.visit_children(node, parent_symbol),
        }
    }

    fn visit_children(&mut self, node: Node<'_>, parent_symbol: Option<SymbolId>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, parent_symbol);
        }
    }

    fn push_declaration_symbol(
        &mut self,
        node: Node<'_>,
        kind: SymbolKind,
        parent_symbol: Option<SymbolId>,
    ) -> SymbolId {
        let symbol_id = SymbolId(self.symbols.len());
        let display_name = node.child_by_field_name("name").map_or_else(
            || node.kind().to_owned(),
            |name| normalized_text(name, self.source),
        );
        let binding_name = node
            .child_by_field_name("identifier")
            .filter(|identifier| identifier.kind() == "identifier")
            .map(|identifier| node_text(identifier, self.source));

        self.symbols.push(Symbol {
            id: symbol_id,
            kind,
            display_name,
            binding_name,
            span: TextSpan::from_node(node),
            parent: parent_symbol,
            syntax_node_kind: node.kind().to_owned(),
        });

        symbol_id
    }

    fn push_relationship_symbol(
        &mut self,
        node: Node<'_>,
        parent_symbol: Option<SymbolId>,
    ) -> Option<SymbolId> {
        let identifier = node.child_by_field_name("identifier")?;
        if identifier.kind() != "identifier" {
            return None;
        }

        let symbol_id = SymbolId(self.symbols.len());
        let binding_name = node_text(identifier, self.source);
        let display_name = node.child_by_field_name("attribute").map_or_else(
            || binding_name.clone(),
            |attribute| normalized_text(attribute, self.source),
        );

        self.symbols.push(Symbol {
            id: symbol_id,
            kind: SymbolKind::Relationship,
            display_name,
            binding_name: Some(binding_name),
            span: TextSpan::from_node(node),
            parent: parent_symbol,
            syntax_node_kind: node.kind().to_owned(),
        });

        Some(symbol_id)
    }

    fn push_relationship_reference(
        &mut self,
        relationship: Node<'_>,
        field_name: &str,
        kind: ReferenceKind,
        target_hint: ReferenceTargetHint,
        containing_symbol: Option<SymbolId>,
    ) {
        let Some(endpoint) = relationship.child_by_field_name(field_name) else {
            return;
        };

        if endpoint.kind() != "identifier" {
            return;
        }

        self.references.push(Reference {
            kind,
            raw_text: node_text(endpoint, self.source),
            span: TextSpan::from_node(endpoint),
            target_hint,
            container_node_kind: relationship.kind().to_owned(),
            containing_symbol,
        });
    }

    fn push_named_deployment_symbol(
        &mut self,
        node: Node<'_>,
        kind: SymbolKind,
        parent_symbol: Option<SymbolId>,
    ) -> Option<SymbolId> {
        let identifier = node.child_by_field_name("identifier")?;
        if identifier.kind() != "identifier" {
            return None;
        }

        let symbol_id = SymbolId(self.symbols.len());
        let display_name = node.child_by_field_name("name").map_or_else(
            || node.kind().to_owned(),
            |name| normalized_text(name, self.source),
        );

        self.symbols.push(Symbol {
            id: symbol_id,
            kind,
            display_name,
            binding_name: Some(node_text(identifier, self.source)),
            span: TextSpan::from_node(node),
            parent: parent_symbol,
            syntax_node_kind: node.kind().to_owned(),
        });

        Some(symbol_id)
    }

    fn push_instance_symbol(
        &mut self,
        node: Node<'_>,
        kind: SymbolKind,
        parent_symbol: Option<SymbolId>,
    ) -> Option<SymbolId> {
        let shape = instance_shape(node);
        let identifier = shape.child_by_field_name("identifier")?;
        if identifier.kind() != "identifier" {
            return None;
        }

        let symbol_id = SymbolId(self.symbols.len());
        let binding_name = node_text(identifier, self.source);
        let display_name = binding_name.clone();

        self.symbols.push(Symbol {
            id: symbol_id,
            kind,
            display_name,
            binding_name: Some(binding_name),
            span: TextSpan::from_node(node),
            parent: parent_symbol,
            syntax_node_kind: node.kind().to_owned(),
        });

        Some(symbol_id)
    }

    fn push_instance_target_reference(
        &mut self,
        instance: Node<'_>,
        containing_symbol: Option<SymbolId>,
    ) {
        let shape = instance_shape(instance);
        let Some(target) = shape.child_by_field_name("target") else {
            return;
        };

        if target.kind() != "identifier" {
            return;
        }

        self.references.push(Reference {
            kind: ReferenceKind::InstanceTarget,
            raw_text: node_text(target, self.source),
            span: TextSpan::from_node(target),
            target_hint: ReferenceTargetHint::Element,
            container_node_kind: instance.kind().to_owned(),
            containing_symbol,
        });
    }

    fn extract_view(
        &mut self,
        view: Node<'_>,
        scope_target_hint: Option<ReferenceTargetHint>,
        include_target_hint: Option<ReferenceTargetHint>,
        animation_target_hint: Option<ReferenceTargetHint>,
        parent_symbol: Option<SymbolId>,
    ) {
        if let Some(target_hint) = scope_target_hint
            && let Some(scope) = view.child_by_field_name("scope")
            && scope.kind() == "identifier"
        {
            self.references.push(Reference {
                kind: ReferenceKind::ViewScope,
                raw_text: node_text(scope, self.source),
                span: TextSpan::from_node(scope),
                target_hint,
                container_node_kind: view.kind().to_owned(),
                containing_symbol: parent_symbol,
            });
        }

        if let Some(body) = view.child_by_field_name("body") {
            if let Some(target_hint) = include_target_hint {
                self.collect_view_include_references(body, view.kind(), target_hint, parent_symbol);
            }
            if let Some(target_hint) = animation_target_hint {
                self.collect_view_animation_references(
                    body,
                    view.kind(),
                    target_hint,
                    parent_symbol,
                );
            }
        }
    }

    fn collect_view_include_references(
        &mut self,
        node: Node<'_>,
        view_kind: &str,
        target_hint: ReferenceTargetHint,
        parent_symbol: Option<SymbolId>,
    ) {
        if node.kind() == "include_statement" {
            let mut cursor = node.walk();
            for value in node.named_children(&mut cursor) {
                self.push_view_reference(
                    value,
                    ReferenceKind::ViewInclude,
                    target_hint,
                    view_kind,
                    parent_symbol,
                );
            }
            return;
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_view_include_references(child, view_kind, target_hint, parent_symbol);
        }
    }

    fn collect_view_animation_references(
        &mut self,
        node: Node<'_>,
        view_kind: &str,
        target_hint: ReferenceTargetHint,
        parent_symbol: Option<SymbolId>,
    ) {
        if node.kind() == "animation_block" {
            let mut cursor = node.walk();
            for value in node.named_children(&mut cursor) {
                self.push_view_reference(
                    value,
                    ReferenceKind::ViewAnimation,
                    target_hint,
                    view_kind,
                    parent_symbol,
                );
            }
            return;
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_view_animation_references(child, view_kind, target_hint, parent_symbol);
        }
    }

    fn push_view_reference(
        &mut self,
        value: Node<'_>,
        kind: ReferenceKind,
        target_hint: ReferenceTargetHint,
        view_kind: &str,
        parent_symbol: Option<SymbolId>,
    ) {
        if value.kind() != "identifier" {
            return;
        }

        self.references.push(Reference {
            kind,
            raw_text: node_text(value, self.source),
            span: TextSpan::from_node(value),
            target_hint,
            container_node_kind: view_kind.to_owned(),
            containing_symbol: parent_symbol,
        });
    }
}

fn relationship_reference_surface(
    relationship: Node<'_>,
) -> (ReferenceKind, ReferenceKind, ReferenceTargetHint) {
    if is_deployment_relationship(relationship) {
        (
            ReferenceKind::DeploymentRelationshipSource,
            ReferenceKind::DeploymentRelationshipDestination,
            ReferenceTargetHint::Deployment,
        )
    } else {
        (
            ReferenceKind::RelationshipSource,
            ReferenceKind::RelationshipDestination,
            ReferenceTargetHint::Element,
        )
    }
}

fn is_deployment_relationship(node: Node<'_>) -> bool {
    let mut current = node;

    while let Some(parent) = current.parent() {
        match parent.kind() {
            "deployment_environment"
            | "deployment_environment_block"
            | "deployment_node"
            | "deployment_node_block"
            | "infrastructure_node"
            | "container_instance"
            | "software_system_instance"
            | "deployment_instance_block" => return true,
            _ => current = parent,
        }
    }

    false
}

fn instance_shape(node: Node<'_>) -> Node<'_> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .next()
        .filter(|child| {
            matches!(
                child.kind(),
                "container_instance_simple"
                    | "container_instance_grouped"
                    | "software_system_instance_simple"
                    | "software_system_instance_grouped"
            )
        })
        .unwrap_or(node)
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

fn normalized_text(node: Node<'_>, source: &str) -> String {
    let raw = node_text(node, source);

    match node.kind() {
        "string" => strip_surrounding_quotes(&raw, "\""),
        "text_block_string" => strip_surrounding_quotes(&raw, "\"\"\""),
        _ => raw,
    }
}

fn strip_surrounding_quotes(raw: &str, delimiter: &str) -> String {
    raw.strip_prefix(delimiter)
        .and_then(|value| value.strip_suffix(delimiter))
        .unwrap_or(raw)
        .to_owned()
}
