//! Declared diagnostic rules for the bounded analysis layer.

use std::sync::OnceLock;

use crate::rule::{DiagnosticSeverity, RuleRegistry, RuleRegistryBuilder, declare_rule};

declare_rule! {
    /// ## What it does
    /// Reports Tree-sitter recovery `ERROR` nodes.
    ///
    /// ## Why is this bad?
    /// An `ERROR` node means the parser could not reconcile the current token
    /// stream with the grammar and had to recover around unexpected syntax.
    pub static SYNTAX_ERROR_NODE = {
        name: "error-node",
        source: "syntax",
        summary: "reports unexpected syntax recovered as tree-sitter error nodes",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports Tree-sitter synthesized `MISSING` nodes.
    ///
    /// ## Why is this bad?
    /// A `MISSING` node means the parser had to invent syntax to continue,
    /// which usually indicates an incomplete or malformed construct in the
    /// source text.
    pub static SYNTAX_MISSING_NODE = {
        name: "missing-node",
        source: "syntax",
        summary: "reports missing syntax recovered by tree-sitter",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports `!include` directives whose local target does not exist.
    ///
    /// ## Why is this bad?
    /// Missing include targets prevent the assembled workspace from loading the
    /// contributor-owned files the document references.
    pub static INCLUDE_MISSING_LOCAL_TARGET = {
        name: "missing-local-target",
        source: "include",
        summary: "reports include directives whose local path does not exist",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports `!include` directives that escape the permitted local subtree.
    ///
    /// ## Why is this bad?
    /// Escaping the allowed subtree breaks the bounded local-loading model and
    /// can pull analysis outside the intended workspace root.
    pub static INCLUDE_ESCAPES_ALLOWED_SUBTREE = {
        name: "escapes-allowed-subtree",
        source: "include",
        summary: "reports include directives that escape the allowed subtree",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports explicit include cycles while assembling a workspace.
    ///
    /// ## Why is this bad?
    /// Include cycles prevent the bounded loader from building one stable
    /// document graph for analysis.
    pub static INCLUDE_CYCLE = {
        name: "cycle",
        source: "include",
        summary: "reports explicit include cycles",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports remote include targets that the bounded loader does not follow.
    ///
    /// ## Why is this bad?
    /// Remote includes remain unresolved in the current local analysis model, so
    /// the user should know the assembled workspace is incomplete.
    pub static INCLUDE_UNSUPPORTED_REMOTE_TARGET = {
        name: "unsupported-remote-target",
        source: "include",
        summary: "reports remote include targets that stay unresolved locally",
        default_severity: DiagnosticSeverity::Warning,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports fatal workspace-load failures that can still be anchored to a source location.
    ///
    /// ## Why is this bad?
    /// Fatal loader failures stop assembled-workspace facts from being built, so
    /// editor users need a visible diagnostic at the directive that caused the
    /// load to abort instead of only seeing stale or missing workspace features.
    pub static WORKSPACE_LOAD_FAILURE = {
        name: "load-failure",
        source: "workspace",
        summary: "reports fatal workspace-load failures with source anchors",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports when more than one declaration claims the same canonical binding.
    ///
    /// ## Why is this bad?
    /// Duplicate bindings make assembled-workspace resolution ambiguous and can
    /// cause navigation or later validation passes to pick the wrong target.
    pub static SEMANTIC_DUPLICATE_BINDING = {
        name: "duplicate-binding",
        source: "semantic",
        summary: "reports duplicate element, deployment, or relationship bindings",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports repeated top-level `model` or `views` sections in one DSL definition.
    ///
    /// ## Why is this bad?
    /// Structurizr accepts standalone fragments for editor workflows, but one
    /// assembled DSL definition still needs at most one `model` section and one
    /// `views` section.
    pub static SEMANTIC_REPEATED_WORKSPACE_SECTION = {
        name: "repeated-workspace-section",
        source: "semantic",
        summary: "reports repeated top-level model or views sections",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports unresolved `!element` selector targets that should name one model or deployment element.
    ///
    /// ## Why is this bad?
    /// When a selector target does not resolve, the bounded analysis layer
    /// cannot anchor the nested block to the intended element.
    pub static SEMANTIC_UNRESOLVED_ELEMENT_SELECTOR = {
        name: "unresolved-element-selector",
        source: "semantic",
        summary: "reports unresolved !element selector targets",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports configuration scopes that conflict with the model depth in the assembled workspace.
    ///
    /// ## Why is this bad?
    /// Scope mismatches mean the workspace declares one modeling boundary while
    /// still containing deeper elements that upstream validation rejects.
    pub static SEMANTIC_WORKSPACE_SCOPE_MISMATCH = {
        name: "workspace-scope-mismatch",
        source: "semantic",
        summary: "reports configuration scopes that conflict with model depth",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports when a supported identifier reference resolves to no known target.
    ///
    /// ## Why is this bad?
    /// An unresolved reference means the assembled workspace is missing a
    /// declaration that later features depend on for navigation and validation.
    pub static SEMANTIC_UNRESOLVED_REFERENCE = {
        name: "unresolved-reference",
        source: "semantic",
        summary: "reports unresolved identifier references",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports when a supported identifier reference resolves ambiguously.
    ///
    /// ## Why is this bad?
    /// Ambiguous references prevent the bounded analysis layer from determining
    /// one reliable target for navigation or later semantic rules.
    pub static SEMANTIC_AMBIGUOUS_REFERENCE = {
        name: "ambiguous-reference",
        source: "semantic",
        summary: "reports identifier references with multiple plausible targets",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports semantic diagnostics that differ across candidate workspace contexts.
    ///
    /// ## Why is this bad?
    /// Shared fragments can participate in more than one workspace. When those
    /// contexts disagree, the analysis layer cannot safely publish the original
    /// error as if it applied unconditionally, but suppressing it entirely hides
    /// useful context from the editor.
    pub static SEMANTIC_MULTI_CONTEXT_DISAGREEMENT = {
        name: "multi-context-disagreement",
        source: "semantic",
        summary: "reports semantic diagnostics that differ across workspace contexts",
        default_severity: DiagnosticSeverity::Warning,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports deployment relationships whose endpoints sit in the same containment chain.
    ///
    /// ## Why is this bad?
    /// Structurizr rejects relationships between deployment parents and their
    /// children because containment already describes that topology.
    pub static SEMANTIC_DEPLOYMENT_PARENT_CHILD_RELATIONSHIP = {
        name: "deployment-parent-child-relationship",
        source: "semantic",
        summary: "reports deployment relationships between parents and children",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports filtered views whose base view already enables automatic layout.
    ///
    /// ## Why is this bad?
    /// Structurizr rejects filtered views derived from auto-layout bases because
    /// the filtered variant cannot safely inherit or override that layout state.
    pub static SEMANTIC_FILTERED_VIEW_AUTOLAYOUT_MISMATCH = {
        name: "filtered-view-autolayout-mismatch",
        source: "semantic",
        summary: "reports filtered views built from auto-layout base views",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports dynamic-view steps whose endpoints do not match any compatible declared relationship.
    ///
    /// ## Why is this bad?
    /// Dynamic views describe runtime behavior in terms of relationships that
    /// already exist in the model, so mismatched steps point at behavior the
    /// assembled workspace never declared.
    pub static SEMANTIC_DYNAMIC_VIEW_RELATIONSHIP_MISMATCH = {
        name: "dynamic-view-relationship-mismatch",
        source: "semantic",
        summary: "reports dynamic-view steps without a matching declared relationship",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports dynamic-view steps that redundantly re-add the view's scoped element.
    ///
    /// ## Why is this bad?
    /// Scoped dynamic views already imply the chosen element, so adding it again
    /// in a step or a referenced relationship is rejected by upstream validation
    /// and usually means the view should be widened to `*` instead.
    pub static SEMANTIC_DYNAMIC_VIEW_SCOPE_REDUNDANCY = {
        name: "dynamic-view-scope-redundancy",
        source: "semantic",
        summary: "reports dynamic-view steps that re-add the scoped element",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports view elements whose kind is incompatible with the current view type.
    ///
    /// ## Why is this bad?
    /// View declarations can only show certain categories of model elements, so
    /// incompatible references are accepted by navigation but rejected by
    /// Structurizr validation.
    pub static SEMANTIC_INVALID_VIEW_ELEMENT = {
        name: "invalid-view-element",
        source: "semantic",
        summary: "reports include or animation elements that the current view type cannot show",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports `!docs` or `!adrs` directives whose local paths are missing or incompatible.
    ///
    /// ## Why is this bad?
    /// File-backed documentation directives participate in the assembled
    /// workspace definition, so missing paths or non-directory ADR targets fail
    /// upstream validation even though the syntax itself still parses cleanly.
    pub static SEMANTIC_INVALID_DOCUMENTATION_PATH = {
        name: "invalid-documentation-path",
        source: "semantic",
        summary: "reports missing or incompatible !docs and !adrs paths",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports image-view sources whose local file inputs are missing or incompatible.
    ///
    /// ## Why is this bad?
    /// Image views can render from checked-in assets or diagram files, so
    /// missing paths and directory/file mismatches leave the workspace unable to
    /// render the declared image source.
    pub static SEMANTIC_INVALID_IMAGE_SOURCE = {
        name: "invalid-image-source",
        source: "semantic",
        summary: "reports missing or incompatible local image-view sources",
        default_severity: DiagnosticSeverity::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports `PlantUML`, `Mermaid`, or `Kroki` image sources that lack their required renderer property.
    ///
    /// ## Why is this bad?
    /// Those image-source families delegate rendering to an external server, so
    /// the DSL needs an explicit view or viewset property such as
    /// `plantuml.url` before the source can resolve successfully.
    pub static SEMANTIC_MISSING_IMAGE_RENDERER_PROPERTY = {
        name: "missing-image-renderer-property",
        source: "semantic",
        summary: "reports image views missing required renderer properties",
        default_severity: DiagnosticSeverity::Error,
    };
}

pub fn register_rules(registry: &mut RuleRegistryBuilder) {
    registry.register(&SYNTAX_ERROR_NODE);
    registry.register(&SYNTAX_MISSING_NODE);
    registry.register(&INCLUDE_MISSING_LOCAL_TARGET);
    registry.register(&INCLUDE_ESCAPES_ALLOWED_SUBTREE);
    registry.register(&INCLUDE_CYCLE);
    registry.register(&INCLUDE_UNSUPPORTED_REMOTE_TARGET);
    registry.register(&WORKSPACE_LOAD_FAILURE);
    registry.register(&SEMANTIC_DUPLICATE_BINDING);
    registry.register(&SEMANTIC_REPEATED_WORKSPACE_SECTION);
    registry.register(&SEMANTIC_UNRESOLVED_ELEMENT_SELECTOR);
    registry.register(&SEMANTIC_UNRESOLVED_REFERENCE);
    registry.register(&SEMANTIC_WORKSPACE_SCOPE_MISMATCH);
    registry.register(&SEMANTIC_AMBIGUOUS_REFERENCE);
    registry.register(&SEMANTIC_MULTI_CONTEXT_DISAGREEMENT);
    registry.register(&SEMANTIC_DEPLOYMENT_PARENT_CHILD_RELATIONSHIP);
    registry.register(&SEMANTIC_FILTERED_VIEW_AUTOLAYOUT_MISMATCH);
    registry.register(&SEMANTIC_DYNAMIC_VIEW_RELATIONSHIP_MISMATCH);
    registry.register(&SEMANTIC_DYNAMIC_VIEW_SCOPE_REDUNDANCY);
    registry.register(&SEMANTIC_INVALID_VIEW_ELEMENT);
    registry.register(&SEMANTIC_INVALID_DOCUMENTATION_PATH);
    registry.register(&SEMANTIC_INVALID_IMAGE_SOURCE);
    registry.register(&SEMANTIC_MISSING_IMAGE_RENDERER_PROPERTY);
}

pub fn diagnostic_rule_registry() -> &'static RuleRegistry {
    static REGISTRY: OnceLock<RuleRegistry> = OnceLock::new();

    REGISTRY.get_or_init(|| {
        let mut builder = RuleRegistryBuilder::default();
        register_rules(&mut builder);
        builder.build()
    })
}

#[cfg(test)]
mod tests {
    use super::diagnostic_rule_registry;

    #[test]
    fn diagnostic_registry_contains_current_rules() {
        let registry = diagnostic_rule_registry();
        let codes = registry
            .all()
            .iter()
            .map(|rule| rule.code())
            .collect::<Vec<_>>();

        assert_eq!(
            codes,
            vec![
                "include.cycle",
                "include.escapes-allowed-subtree",
                "include.missing-local-target",
                "include.unsupported-remote-target",
                "semantic.ambiguous-reference",
                "semantic.deployment-parent-child-relationship",
                "semantic.duplicate-binding",
                "semantic.dynamic-view-relationship-mismatch",
                "semantic.dynamic-view-scope-redundancy",
                "semantic.filtered-view-autolayout-mismatch",
                "semantic.invalid-documentation-path",
                "semantic.invalid-image-source",
                "semantic.invalid-view-element",
                "semantic.missing-image-renderer-property",
                "semantic.multi-context-disagreement",
                "semantic.repeated-workspace-section",
                "semantic.unresolved-element-selector",
                "semantic.unresolved-reference",
                "semantic.workspace-scope-mismatch",
                "syntax.error-node",
                "syntax.missing-node",
                "workspace.load-failure",
            ]
        );
        assert!(registry.get("syntax.error-node").is_some());
        assert!(registry.get("syntax.missing-node").is_some());
        assert!(registry.get("workspace.load-failure").is_some());
        assert!(registry.get("include.cycle").is_some());
        assert!(registry.get("include.escapes-allowed-subtree").is_some());
        assert!(registry.get("include.missing-local-target").is_some());
        assert!(registry.get("include.unsupported-remote-target").is_some());
        assert!(registry.get("semantic.duplicate-binding").is_some());
        assert!(
            registry
                .get("semantic.multi-context-disagreement")
                .is_some()
        );
        assert!(
            registry
                .get("semantic.dynamic-view-relationship-mismatch")
                .is_some()
        );
        assert!(
            registry
                .get("semantic.dynamic-view-scope-redundancy")
                .is_some()
        );
        assert!(
            registry
                .get("semantic.filtered-view-autolayout-mismatch")
                .is_some()
        );
        assert!(
            registry
                .get("semantic.invalid-documentation-path")
                .is_some()
        );
        assert!(registry.get("semantic.invalid-image-source").is_some());
        assert!(registry.get("semantic.invalid-view-element").is_some());
        assert!(
            registry
                .get("semantic.missing-image-renderer-property")
                .is_some()
        );
        assert!(
            registry
                .get("semantic.multi-context-disagreement")
                .is_some()
        );
        assert!(
            registry
                .get("semantic.repeated-workspace-section")
                .is_some()
        );
        assert!(
            registry
                .get("semantic.unresolved-element-selector")
                .is_some()
        );
        assert!(registry.get("semantic.unresolved-reference").is_some());
        assert!(registry.get("semantic.workspace-scope-mismatch").is_some());
        assert!(registry.get("semantic.ambiguous-reference").is_some());
    }
}
