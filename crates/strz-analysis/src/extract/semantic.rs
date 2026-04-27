//! Extraction of higher-level syntax-backed facts for later semantic validators.

use tree_sitter::{Node, Tree};

use crate::semantic::{
    AutoLayoutFact, ConfigurationScopeFact, DynamicRelationshipFact,
    DynamicRelationshipReferenceFact, DynamicViewStepFact, ElementDirectiveFact, ImageSourceFact,
    ImageSourceKind, ImageSourceMode, PropertyFact, RelationshipFact, ResourceDirectiveFact,
    ResourceDirectiveKind, ValueFact, ViewFact, ViewKind, WorkspaceScope, WorkspaceSectionFact,
    WorkspaceSectionKind,
};
use crate::span::TextSpan;

#[allow(clippy::redundant_pub_crate)]
#[derive(Debug, Default)]
pub(crate) struct CollectedSemanticFacts {
    pub(crate) workspace_sections: Vec<WorkspaceSectionFact>,
    pub(crate) configuration_scopes: Vec<ConfigurationScopeFact>,
    pub(crate) property_facts: Vec<PropertyFact>,
    pub(crate) resource_directives: Vec<ResourceDirectiveFact>,
    pub(crate) element_directives: Vec<ElementDirectiveFact>,
    pub(crate) relationship_facts: Vec<RelationshipFact>,
    pub(crate) view_facts: Vec<ViewFact>,
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn collect(tree: &Tree, source: &str) -> CollectedSemanticFacts {
    let mut collector = SemanticCollector {
        source,
        facts: CollectedSemanticFacts::default(),
    };
    collector.visit(tree.root_node());
    collector.facts
}

struct SemanticCollector<'a> {
    source: &'a str,
    facts: CollectedSemanticFacts,
}

impl SemanticCollector<'_> {
    fn visit(&mut self, node: Node<'_>) {
        // Gather the higher-level fact packet in one tree walk so later workspace
        // validators can build on stable, source-ordered structures instead of
        // reinterpreting raw syntax ad hoc in `workspace.rs`.
        match node.kind() {
            "model" => self
                .facts
                .workspace_sections
                .push(Self::workspace_section(node, WorkspaceSectionKind::Model)),
            "views" => self
                .facts
                .workspace_sections
                .push(Self::workspace_section(node, WorkspaceSectionKind::Views)),
            "configuration" => self.facts.workspace_sections.push(Self::workspace_section(
                node,
                WorkspaceSectionKind::Configuration,
            )),
            "scope_statement"
                if node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "configuration_block") =>
            {
                if let Some(scope_fact) = self.configuration_scope(node) {
                    self.facts.configuration_scopes.push(scope_fact);
                }
            }
            "property_entry" => {
                if let Some(property_fact) = self.property_fact(node) {
                    self.facts.property_facts.push(property_fact);
                }
            }
            "docs_directive" | "adrs_directive" => {
                if let Some(resource_fact) = self.resource_directive(node) {
                    self.facts.resource_directives.push(resource_fact);
                }
            }
            "element_directive" => {
                if let Some(element_fact) = self.element_directive(node) {
                    self.facts.element_directives.push(element_fact);
                }
            }
            "relationship" | "nested_relationship" => {
                if let Some(relationship_fact) = self.relationship_fact(node) {
                    self.facts.relationship_facts.push(relationship_fact);
                }
            }
            "system_landscape_view"
            | "system_context_view"
            | "container_view"
            | "component_view"
            | "filtered_view"
            | "dynamic_view"
            | "deployment_view"
            | "custom_view"
            | "image_view" => self.facts.view_facts.push(self.view_fact(node)),
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child);
        }
    }

    fn workspace_section(node: Node<'_>, kind: WorkspaceSectionKind) -> WorkspaceSectionFact {
        WorkspaceSectionFact {
            kind,
            span: TextSpan::from_node(node),
        }
    }

    fn configuration_scope(&self, node: Node<'_>) -> Option<ConfigurationScopeFact> {
        let value = self.value_from_field(node, "value")?;
        Some(ConfigurationScopeFact {
            scope: WorkspaceScope::from_raw(&value.normalized_text),
            value,
            span: TextSpan::from_node(node),
        })
    }

    fn property_fact(&self, node: Node<'_>) -> Option<PropertyFact> {
        Some(PropertyFact {
            name: self.value_from_field(node, "name")?,
            value: self.value_from_field(node, "value")?,
            span: TextSpan::from_node(node),
            // Skip the immediate `properties { ... }` wrapper so consumers see
            // the owning block kind (`views_block`, `workspace_block`, etc.).
            container_node_kind: enclosing_parent_kind(node, &["properties_block"]),
        })
    }

    fn resource_directive(&self, node: Node<'_>) -> Option<ResourceDirectiveFact> {
        let kind = match node.kind() {
            "docs_directive" => ResourceDirectiveKind::Docs,
            "adrs_directive" => ResourceDirectiveKind::Adrs,
            _ => return None,
        };

        Some(ResourceDirectiveFact {
            kind,
            path: self.value_from_field(node, "path")?,
            importer: self.value_from_field(node, "importer"),
            span: TextSpan::from_node(node),
            container_node_kind: direct_parent_kind(node),
        })
    }

    fn element_directive(&self, node: Node<'_>) -> Option<ElementDirectiveFact> {
        Some(ElementDirectiveFact {
            target: self.value_from_field(node, "target")?,
            span: TextSpan::from_node(node),
            container_node_kind: direct_parent_kind(node),
        })
    }

    fn relationship_fact(&self, node: Node<'_>) -> Option<RelationshipFact> {
        let mut cursor = node.walk();
        let mut attributes = node
            .children_by_field_name("attribute", &mut cursor)
            .map(|attribute| self.value_from_node(attribute))
            .filter(|value| !value.normalized_text.is_empty());

        Some(RelationshipFact {
            span: TextSpan::from_node(node),
            source: self.value_from_field(node, "source"),
            destination: self.value_from_field(node, "destination")?,
            description: attributes.next(),
            technology: attributes.next(),
        })
    }

    fn view_fact(&self, node: Node<'_>) -> ViewFact {
        let body = node.child_by_field_name("body");
        let mut include_values = Vec::new();
        let mut exclude_values = Vec::new();
        let mut animation_values = Vec::new();
        let mut dynamic_steps = Vec::new();
        let mut image_sources = Vec::new();

        if let Some(body_node) = body {
            self.collect_statement_values(body_node, "include_statement", &mut include_values);
            self.collect_statement_values(body_node, "exclude_statement", &mut exclude_values);
            self.collect_statement_values(body_node, "animation_block", &mut animation_values);
            self.collect_dynamic_steps(body_node, &mut dynamic_steps);
            self.collect_image_sources(body_node, ImageSourceMode::Default, &mut image_sources);
        }

        ViewFact {
            kind: view_kind(node.kind()).expect("BUG: matched node kind should map to view kind"),
            span: TextSpan::from_node(node),
            body_span: body.map(TextSpan::from_node),
            key: self.value_from_field(node, "key"),
            scope: self.value_from_field(node, "scope"),
            environment: self.value_from_field(node, "environment"),
            base_key: self.value_from_field(node, "base_key"),
            filter_mode: node
                .child_by_field_name("mode")
                .map(|mode| node_text(mode, self.source)),
            filter_tags: self.value_from_field(node, "tags"),
            auto_layout: body.and_then(|body_node| self.first_auto_layout(body_node)),
            include_values,
            exclude_values,
            animation_values,
            dynamic_steps,
            image_sources,
        }
    }

    fn first_auto_layout(&self, node: Node<'_>) -> Option<AutoLayoutFact> {
        if node.kind() == "auto_layout_statement" {
            return Some(AutoLayoutFact {
                span: TextSpan::from_node(node),
                direction: node
                    .child_by_field_name("direction")
                    .map(|value| node_text(value, self.source)),
                rank_separation: node
                    .child_by_field_name("rank_separation")
                    .map(|value| node_text(value, self.source)),
                node_separation: node
                    .child_by_field_name("node_separation")
                    .map(|value| node_text(value, self.source)),
            });
        }

        // Tree-sitter recovery can wrap a valid `autoLayout` under intermediate
        // error nodes, so keep walking until we find the first real statement
        // rather than assuming one fixed child shape for every view body.
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if let Some(auto_layout) = self.first_auto_layout(child) {
                return Some(auto_layout);
            }
        }

        None
    }

    fn collect_statement_values(
        &self,
        node: Node<'_>,
        statement_kind: &str,
        values: &mut Vec<ValueFact>,
    ) {
        // Recovery can wrap otherwise-valid statements under intermediate error
        // nodes, so keep descending until we find the statement we care about
        // instead of assuming one flat view-body shape.
        if node.kind() == statement_kind {
            let mut cursor = node.walk();
            for value in node.named_children(&mut cursor) {
                values.push(self.value_from_node(value));
            }
            return;
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_statement_values(child, statement_kind, values);
        }
    }

    fn collect_dynamic_steps(&self, node: Node<'_>, steps: &mut Vec<DynamicViewStepFact>) {
        // Dynamic steps need the same recovery-friendly descent because partially
        // typed relationships may still be nested under `ERROR` nodes while
        // remaining useful to later semantic tooling.
        match node.kind() {
            "dynamic_relationship" => {
                if let Some(step) = self.dynamic_relationship(node) {
                    steps.push(DynamicViewStepFact::Relationship(Box::new(step)));
                }
            }
            "dynamic_relationship_reference" => {
                if let Some(step) = self.dynamic_relationship_reference(node) {
                    steps.push(DynamicViewStepFact::RelationshipReference(Box::new(step)));
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_dynamic_steps(child, steps);
        }
    }

    fn dynamic_relationship(&self, node: Node<'_>) -> Option<DynamicRelationshipFact> {
        Some(DynamicRelationshipFact {
            span: TextSpan::from_node(node),
            order: node
                .child_by_field_name("order")
                .map(|value| node_text(value, self.source)),
            source: self.value_from_field(node, "source")?,
            destination: self.value_from_field(node, "destination")?,
            description: self.value_from_field(node, "description"),
            technology: self.value_from_field(node, "technology"),
        })
    }

    fn dynamic_relationship_reference(
        &self,
        node: Node<'_>,
    ) -> Option<DynamicRelationshipReferenceFact> {
        Some(DynamicRelationshipReferenceFact {
            span: TextSpan::from_node(node),
            order: node
                .child_by_field_name("order")
                .map(|value| node_text(value, self.source)),
            relationship: self.value_from_field(node, "relationship")?,
            description: self.value_from_field(node, "description")?,
        })
    }

    fn collect_image_sources(
        &self,
        node: Node<'_>,
        mode: ImageSourceMode,
        sources: &mut Vec<ImageSourceFact>,
    ) {
        match node.kind() {
            "light_image_sources" => {
                if let Some(body) = node.child_by_field_name("body") {
                    self.collect_image_sources(body, ImageSourceMode::Light, sources);
                }
                return;
            }
            "dark_image_sources" => {
                if let Some(body) = node.child_by_field_name("body") {
                    self.collect_image_sources(body, ImageSourceMode::Dark, sources);
                }
                return;
            }
            "plantuml_source" | "mermaid_source" | "kroki_source" | "image_source" => {
                if let Some(source_fact) = self.image_source(node, mode) {
                    sources.push(source_fact);
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_image_sources(child, mode, sources);
        }
    }

    fn image_source(&self, node: Node<'_>, mode: ImageSourceMode) -> Option<ImageSourceFact> {
        let (kind, format) = match node.kind() {
            "plantuml_source" => (ImageSourceKind::PlantUml, None),
            "mermaid_source" => (ImageSourceKind::Mermaid, None),
            "kroki_source" => (
                ImageSourceKind::Kroki,
                self.value_from_field(node, "format"),
            ),
            "image_source" => (ImageSourceKind::Image, None),
            _ => return None,
        };

        Some(ImageSourceFact {
            kind,
            mode,
            format,
            value: self.value_from_field(node, "value")?,
            span: TextSpan::from_node(node),
        })
    }

    fn value_from_field(&self, node: Node<'_>, field: &str) -> Option<ValueFact> {
        node.child_by_field_name(field)
            .map(|value| self.value_from_node(value))
    }

    fn value_from_node(&self, node: Node<'_>) -> ValueFact {
        ValueFact::new(
            node_text(node, self.source),
            crate::includes::DirectiveValueKind::from_node_kind(node.kind()),
            TextSpan::from_node(node),
        )
    }
}

fn view_kind(node_kind: &str) -> Option<ViewKind> {
    match node_kind {
        "system_landscape_view" => Some(ViewKind::SystemLandscape),
        "system_context_view" => Some(ViewKind::SystemContext),
        "container_view" => Some(ViewKind::Container),
        "component_view" => Some(ViewKind::Component),
        "filtered_view" => Some(ViewKind::Filtered),
        "dynamic_view" => Some(ViewKind::Dynamic),
        "deployment_view" => Some(ViewKind::Deployment),
        "custom_view" => Some(ViewKind::Custom),
        "image_view" => Some(ViewKind::Image),
        _ => None,
    }
}

fn direct_parent_kind(node: Node<'_>) -> String {
    node.parent().map_or_else(
        || "source_file".to_owned(),
        |parent| parent.kind().to_owned(),
    )
}

fn enclosing_parent_kind(node: Node<'_>, skipped_kinds: &[&str]) -> String {
    let mut current = node;

    while let Some(parent) = current.parent() {
        if skipped_kinds.contains(&parent.kind()) {
            current = parent;
            continue;
        }
        return parent.kind().to_owned();
    }

    "source_file".to_owned()
}

fn node_text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .expect("node text should be utf-8")
        .to_owned()
}
