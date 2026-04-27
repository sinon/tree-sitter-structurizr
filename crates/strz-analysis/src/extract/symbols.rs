//! Handwritten extraction for bounded-MVP identifier modes, symbols, and references.

use std::collections::BTreeMap;

use tree_sitter::{Node, Tree};

use crate::includes::{DirectiveContainer, DirectiveValueKind};
use crate::span::TextSpan;
use crate::symbols::{
    IdentifierMode, IdentifierModeFact, Reference, ReferenceKind, ReferenceTargetHint, Symbol,
    SymbolId, SymbolKind,
};
use crate::{TagSurface, tag_surface_for_node_kind};

pub fn collect_identifier_modes(tree: &Tree, source: &str) -> Vec<IdentifierModeFact> {
    let mut facts = Vec::new();
    collect_identifier_mode_from_node(tree.root_node(), source, &mut facts);
    facts
}

pub fn collect_tags(tree: &Tree, source: &str) -> Vec<String> {
    let mut tags = Vec::new();
    collect_tags_from_node(tree.root_node(), source, &mut tags);
    tags
}

pub fn collect_symbols_and_references(tree: &Tree, source: &str) -> (Vec<Symbol>, Vec<Reference>) {
    // Keep symbol and reference extraction in one pass so snapshots see a
    // consistent view of declaration hierarchy and cross-reference sites.
    let mut extractor = SymbolExtractor::new(source, collect_supported_archetype_symbol_kinds(tree, source));
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

fn collect_tags_from_node(node: Node<'_>, source: &str, tags: &mut Vec<String>) {
    if let Some(surface) = tag_surface_for_node_kind(node.kind()) {
        collect_tags_from_surface(node, surface, source, tags);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_tags_from_node(child, source, tags);
    }
}

fn collect_tags_from_surface(
    node: Node<'_>,
    surface: TagSurface,
    source: &str,
    tags: &mut Vec<String>,
) {
    match surface {
        TagSurface::TagStatement => {
            if let Some(tag) = metadata_value(node, source) {
                extend_tags(tags, &tag);
            }
        }
        TagSurface::TagsStatement => {
            for tag_value in metadata_values(node, source) {
                extend_tags(tags, &tag_value);
            }
        }
        TagSurface::NamedField { field_name, .. } => {
            if let Some(tag_value) = normalized_nonempty_field(node, field_name, source) {
                extend_tags(tags, &tag_value);
            }
        }
        TagSurface::IndexedField {
            field_name,
            index,
            excluded_kind,
            ..
        } => {
            let tag_value = excluded_kind.map_or_else(
                || nth_field_value(node, field_name, source, index),
                |excluded_kind| {
                    nth_field_value_excluding(node, field_name, source, index, excluded_kind)
                },
            );
            if let Some(tag_value) = tag_value {
                extend_tags(tags, &tag_value);
            }
        }
    }
}

#[derive(Debug, Default)]
struct ExtractedSymbolMetadata {
    description: Option<String>,
    technology: Option<String>,
    tags: Vec<String>,
    url: Option<String>,
}

struct ExtractedBinding {
    name: String,
    span: TextSpan,
}

struct SymbolExtractor<'a> {
    source: &'a str,
    archetype_symbol_kinds: BTreeMap<String, SymbolKind>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
}

impl<'a> SymbolExtractor<'a> {
    const fn new(source: &'a str, archetype_symbol_kinds: BTreeMap<String, SymbolKind>) -> Self {
        Self {
            source,
            archetype_symbol_kinds,
            symbols: Vec::new(),
            references: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node<'_>, parent_symbol: Option<SymbolId>) {
        // Declaration nodes build the hierarchical symbol tree, while
        // relationships and views contribute reference edges into that tree.
        if let Some(kind) = element_symbol_kind(node.kind()) {
            let symbol_id = self.push_declaration_symbol(node, kind, parent_symbol);
            self.visit_children(node, Some(symbol_id));
            return;
        }
        if let Some(kind) = archetype_instance_symbol_kind(node, &self.archetype_symbol_kinds, self.source) {
            let symbol_id = self.push_declaration_symbol(node, kind, parent_symbol);
            self.visit_children(node, Some(symbol_id));
            return;
        }
        if let Some(kind) = named_deployment_symbol_kind(node.kind()) {
            let symbol_id = self.push_named_deployment_symbol(node, kind, parent_symbol);
            self.visit_children(node, symbol_id.or(parent_symbol));
            return;
        }
        if let Some(kind) = instance_symbol_kind(node.kind()) {
            let symbol_id = self.push_instance_symbol(node, kind, parent_symbol);
            self.push_instance_target_reference(node, symbol_id.or(parent_symbol));
            self.visit_children(node, symbol_id.or(parent_symbol));
            return;
        }

        match node.kind() {
            "relationship" => {
                let relationship_symbol = self.push_relationship_symbol(node, parent_symbol);
                let containing_symbol = relationship_symbol.or(parent_symbol);
                self.collect_relationship_references(node, containing_symbol);
                self.visit_children(node, containing_symbol);
            }
            "nested_relationship" => {
                self.collect_relationship_references(node, parent_symbol);
                self.visit_children(node, parent_symbol);
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
            "dynamic_view" => self.extract_dynamic_view(node, parent_symbol),
            _ => self.visit_children(node, parent_symbol),
        }
    }

    fn visit_children(&mut self, node: Node<'_>, parent_symbol: Option<SymbolId>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, parent_symbol);
        }
    }

    fn push_symbol(
        &mut self,
        node: Node<'_>,
        kind: SymbolKind,
        display_name: String,
        binding: Option<ExtractedBinding>,
        metadata: ExtractedSymbolMetadata,
        parent_symbol: Option<SymbolId>,
    ) -> SymbolId {
        let symbol_id = SymbolId(self.symbols.len());
        let ExtractedSymbolMetadata {
            description,
            technology,
            tags,
            url,
        } = metadata;
        let (binding_name, binding_span) = binding.map_or((None, None), |binding| {
            (Some(binding.name), Some(binding.span))
        });

        self.symbols.push(Symbol {
            id: symbol_id,
            kind,
            display_name,
            binding_name,
            binding_span,
            description,
            technology,
            tags,
            url,
            span: TextSpan::from_node(node),
            parent: parent_symbol,
            syntax_node_kind: node.kind().to_owned(),
        });

        symbol_id
    }

    fn push_declaration_symbol(
        &mut self,
        node: Node<'_>,
        kind: SymbolKind,
        parent_symbol: Option<SymbolId>,
    ) -> SymbolId {
        let display_name = node.child_by_field_name("name").map_or_else(
            || node.kind().to_owned(),
            |name| normalized_text(name, self.source),
        );
        let metadata = declaration_metadata(node, self.source);
        let binding = node
            .child_by_field_name("identifier")
            .filter(|identifier| identifier.kind() == "identifier")
            .map(|identifier| ExtractedBinding {
                name: node_text(identifier, self.source),
                span: TextSpan::from_node(identifier),
            });

        self.push_symbol(node, kind, display_name, binding, metadata, parent_symbol)
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

        let binding_name = node_text(identifier, self.source);
        let display_name = node
            .child_by_field_name("attribute")
            .and_then(|attribute| normalized_nonempty_text(attribute, self.source))
            .unwrap_or_else(|| binding_name.clone());
        let metadata = declaration_metadata(node, self.source);

        Some(self.push_symbol(
            node,
            SymbolKind::Relationship,
            display_name,
            Some(ExtractedBinding {
                name: binding_name,
                span: TextSpan::from_node(identifier),
            }),
            metadata,
            parent_symbol,
        ))
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

        if !matches!(endpoint.kind(), "identifier" | "this_keyword") {
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

    fn collect_relationship_references(
        &mut self,
        relationship: Node<'_>,
        containing_symbol: Option<SymbolId>,
    ) {
        let (source_kind, destination_kind, target_hint) =
            relationship_reference_surface(relationship);
        self.push_relationship_reference(
            relationship,
            "source",
            source_kind,
            target_hint,
            containing_symbol,
        );
        self.push_relationship_reference(
            relationship,
            "destination",
            destination_kind,
            target_hint,
            containing_symbol,
        );
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

        let display_name = node.child_by_field_name("name").map_or_else(
            || node.kind().to_owned(),
            |name| normalized_text(name, self.source),
        );
        let metadata = declaration_metadata(node, self.source);

        Some(self.push_symbol(
            node,
            kind,
            display_name,
            Some(ExtractedBinding {
                name: node_text(identifier, self.source),
                span: TextSpan::from_node(identifier),
            }),
            metadata,
            parent_symbol,
        ))
    }

    fn push_instance_symbol(
        &mut self,
        node: Node<'_>,
        kind: SymbolKind,
        parent_symbol: Option<SymbolId>,
    ) -> Option<SymbolId> {
        let identifier = node.child_by_field_name("identifier")?;
        if identifier.kind() != "identifier" {
            return None;
        }

        let binding_name = node_text(identifier, self.source);
        let display_name = binding_name.clone();
        let metadata = declaration_metadata(node, self.source);

        Some(self.push_symbol(
            node,
            kind,
            display_name,
            Some(ExtractedBinding {
                name: binding_name,
                span: TextSpan::from_node(identifier),
            }),
            metadata,
            parent_symbol,
        ))
    }

    fn push_instance_target_reference(
        &mut self,
        instance: Node<'_>,
        containing_symbol: Option<SymbolId>,
    ) {
        let Some(target) = instance.child_by_field_name("target") else {
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
                // `exclude` participates in the same bounded identifier surface
                // as `include`, so both must flow through the shared reference
                // table for navigation and rename to stay aligned.
                self.collect_view_statement_references(
                    body,
                    "include_statement",
                    ReferenceKind::ViewInclude,
                    view.kind(),
                    target_hint,
                    parent_symbol,
                );
                self.collect_view_statement_references(
                    body,
                    "exclude_statement",
                    ReferenceKind::ViewExclude,
                    view.kind(),
                    target_hint,
                    parent_symbol,
                );
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

    fn extract_dynamic_view(&mut self, view: Node<'_>, parent_symbol: Option<SymbolId>) {
        self.extract_view(
            view,
            Some(ReferenceTargetHint::Element),
            None,
            None,
            parent_symbol,
        );

        if let Some(body) = view.child_by_field_name("body") {
            self.collect_dynamic_relationship_references(body, parent_symbol);
        }
    }

    fn collect_view_statement_references(
        &mut self,
        node: Node<'_>,
        statement_kind: &str,
        reference_kind: ReferenceKind,
        view_kind: &str,
        target_hint: ReferenceTargetHint,
        parent_symbol: Option<SymbolId>,
    ) {
        if node.kind() == statement_kind {
            let mut cursor = node.walk();
            for value in node.named_children(&mut cursor) {
                self.push_view_reference(
                    value,
                    reference_kind,
                    target_hint,
                    view_kind,
                    parent_symbol,
                );
            }
            return;
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_view_statement_references(
                child,
                statement_kind,
                reference_kind,
                view_kind,
                target_hint,
                parent_symbol,
            );
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

    fn collect_dynamic_relationship_references(
        &mut self,
        node: Node<'_>,
        parent_symbol: Option<SymbolId>,
    ) {
        if node.kind() == "dynamic_relationship" {
            let (source_kind, destination_kind, target_hint) = relationship_reference_surface(node);
            self.push_relationship_reference(
                node,
                "source",
                source_kind,
                target_hint,
                parent_symbol,
            );
            self.push_relationship_reference(
                node,
                "destination",
                destination_kind,
                target_hint,
                parent_symbol,
            );
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_dynamic_relationship_references(child, parent_symbol);
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

fn element_symbol_kind(node_kind: &str) -> Option<SymbolKind> {
    match node_kind {
        "person" => Some(SymbolKind::Person),
        "software_system" => Some(SymbolKind::SoftwareSystem),
        "container" => Some(SymbolKind::Container),
        "component" => Some(SymbolKind::Component),
        _ => None,
    }
}

fn collect_supported_archetype_symbol_kinds(
    tree: &Tree,
    source: &str,
) -> BTreeMap<String, SymbolKind> {
    let mut definitions = Vec::<(String, String)>::new();
    collect_archetype_definitions(tree.root_node(), source, &mut definitions);

    let mut resolved = BTreeMap::<String, SymbolKind>::new();
    let mut changed = true;
    while changed {
        changed = false;
        for (identifier, base) in &definitions {
            if resolved.contains_key(identifier) {
                continue;
            }
            let Some(kind) = resolve_archetype_base_symbol_kind(base, &resolved) else {
                continue;
            };
            resolved.insert(identifier.clone(), kind);
            changed = true;
        }
    }

    resolved
}

fn collect_archetype_definitions(
    node: Node<'_>,
    source: &str,
    definitions: &mut Vec<(String, String)>,
) {
    if node.kind() == "archetype_definition"
        && let (Some(identifier), Some(base)) = (
            node.child_by_field_name("identifier"),
            node.child_by_field_name("base"),
        )
    {
        definitions.push((node_text(identifier, source), normalized_text(base, source)));
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_archetype_definitions(child, source, definitions);
    }
}

fn resolve_archetype_base_symbol_kind(
    base: &str,
    resolved: &BTreeMap<String, SymbolKind>,
) -> Option<SymbolKind> {
    match base.to_ascii_lowercase().as_str() {
        "person" => Some(SymbolKind::Person),
        "softwaresystem" => Some(SymbolKind::SoftwareSystem),
        "container" => Some(SymbolKind::Container),
        "component" => Some(SymbolKind::Component),
        _ => resolved.get(base).copied(),
    }
}

fn archetype_instance_symbol_kind(
    node: Node<'_>,
    archetype_symbol_kinds: &BTreeMap<String, SymbolKind>,
    source: &str,
) -> Option<SymbolKind> {
    if node.kind() != "archetype_instance" {
        return None;
    }

    let archetype_name = node.child_by_field_name("kind")?;
    archetype_symbol_kinds
        .get(&node_text(archetype_name, source))
        .copied()
}

fn named_deployment_symbol_kind(node_kind: &str) -> Option<SymbolKind> {
    match node_kind {
        "deployment_environment" => Some(SymbolKind::DeploymentEnvironment),
        "deployment_node" => Some(SymbolKind::DeploymentNode),
        "infrastructure_node" => Some(SymbolKind::InfrastructureNode),
        _ => None,
    }
}

fn instance_symbol_kind(node_kind: &str) -> Option<SymbolKind> {
    match node_kind {
        "container_instance" => Some(SymbolKind::ContainerInstance),
        "software_system_instance" => Some(SymbolKind::SoftwareSystemInstance),
        _ => None,
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

fn declaration_metadata(node: Node<'_>, source: &str) -> ExtractedSymbolMetadata {
    match node.kind() {
        "person" | "software_system" | "container" | "component" | "archetype_instance" => {
            element_metadata(node, source)
        }
        "relationship" => relationship_metadata(node, source),
        "deployment_environment" | "deployment_node" | "infrastructure_node" => {
            deployment_metadata(node, source)
        }
        "container_instance" | "software_system_instance" => instance_metadata(node, source),
        _ => ExtractedSymbolMetadata::default(),
    }
}

fn element_metadata(node: Node<'_>, source: &str) -> ExtractedSymbolMetadata {
    let mut metadata = ExtractedSymbolMetadata {
        description: normalized_nonempty_field(node, "description", source),
        technology: normalized_nonempty_field(node, "technology", source),
        tags: Vec::new(),
        url: None,
    };

    if let Some(tags) = normalized_nonempty_field(node, "tags", source) {
        extend_tags(&mut metadata.tags, &tags);
    }
    if let Some(body) = node.child_by_field_name("body") {
        apply_body_metadata(&mut metadata, body, source);
    }

    metadata
}

fn relationship_metadata(node: Node<'_>, source: &str) -> ExtractedSymbolMetadata {
    let mut metadata = ExtractedSymbolMetadata::default();
    let mut cursor = node.walk();
    let mut attributes = node
        .children_by_field_name("attribute", &mut cursor)
        .map(|attribute| normalized_text(attribute, source));

    let _ = attributes.next();
    metadata.technology = nonempty_text(attributes.next());
    if let Some(tags) = nonempty_text(attributes.next()) {
        extend_tags(&mut metadata.tags, &tags);
    }
    if let Some(body) = node.child_by_field_name("body") {
        apply_body_metadata(&mut metadata, body, source);
    }

    metadata
}

fn deployment_metadata(node: Node<'_>, source: &str) -> ExtractedSymbolMetadata {
    let mut metadata = ExtractedSymbolMetadata::default();
    let mut cursor = node.walk();
    let mut attributes = node
        .children_by_field_name("attribute", &mut cursor)
        .filter(|attribute| attribute.kind() != "number")
        .map(|attribute| normalized_text(attribute, source));

    // Deployment nodes store positional strings more generically than model
    // elements, so we preserve the common description/technology/tags ordering
    // without pretending the grammar already models each slot semantically.
    metadata.description = nonempty_text(attributes.next());
    metadata.technology = nonempty_text(attributes.next());
    if let Some(tags) = nonempty_text(attributes.next()) {
        extend_tags(&mut metadata.tags, &tags);
    }
    if let Some(body) = node.child_by_field_name("body") {
        apply_body_metadata(&mut metadata, body, source);
    }

    metadata
}

fn instance_metadata(node: Node<'_>, source: &str) -> ExtractedSymbolMetadata {
    let mut metadata = ExtractedSymbolMetadata::default();
    if let Some(tags) = normalized_nonempty_field(node, "tags", source) {
        extend_tags(&mut metadata.tags, &tags);
    }
    if let Some(body) = node.child_by_field_name("body") {
        apply_body_metadata(&mut metadata, body, source);
    }
    metadata
}

fn apply_body_metadata(metadata: &mut ExtractedSymbolMetadata, body: Node<'_>, source: &str) {
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        match child.kind() {
            "description_statement" => {
                metadata.description = metadata_value(child, source);
            }
            "technology_statement" => {
                metadata.technology = metadata_value(child, source);
            }
            "tag_statement" => {
                if let Some(tag) = metadata_value(child, source) {
                    extend_tags(&mut metadata.tags, &tag);
                }
            }
            "tags_statement" => {
                for tag_value in metadata_values(child, source) {
                    extend_tags(&mut metadata.tags, &tag_value);
                }
            }
            "url_statement" => {
                metadata.url = metadata_value(child, source);
            }
            _ => {}
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

fn normalized_nonempty_field(node: Node<'_>, field_name: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field_name)
        .and_then(|field| normalized_nonempty_text(field, source))
}

fn metadata_value(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("value")
        .and_then(|value| normalized_nonempty_text(value, source))
}

fn metadata_values(node: Node<'_>, source: &str) -> Vec<String> {
    let mut cursor = node.walk();
    node.children_by_field_name("value", &mut cursor)
        .filter_map(|value| normalized_nonempty_text(value, source))
        .collect()
}

fn nth_field_value(node: Node<'_>, field_name: &str, source: &str, index: usize) -> Option<String> {
    let mut cursor = node.walk();
    nonempty_text(
        node.children_by_field_name(field_name, &mut cursor)
            .map(|child| normalized_text(child, source))
            .nth(index),
    )
}

fn nth_field_value_excluding(
    node: Node<'_>,
    field_name: &str,
    source: &str,
    index: usize,
    excluded_kind: &str,
) -> Option<String> {
    let mut cursor = node.walk();
    nonempty_text(
        node.children_by_field_name(field_name, &mut cursor)
            .filter(|child| child.kind() != excluded_kind)
            .map(|child| normalized_text(child, source))
            .nth(index),
    )
}

fn nonempty_text(text: Option<String>) -> Option<String> {
    text.filter(|text| !text.is_empty())
}

fn normalized_nonempty_text(node: Node<'_>, source: &str) -> Option<String> {
    let text = normalized_text(node, source);
    nonempty_text(Some(text))
}

fn extend_tags(tags: &mut Vec<String>, raw_value: &str) {
    for tag in raw_value
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        if !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.to_owned());
        }
    }
}
