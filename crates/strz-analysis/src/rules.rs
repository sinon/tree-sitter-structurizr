//! Declared diagnostic rules for the bounded analysis layer.

use std::sync::OnceLock;

use crate::rule::{Level, RuleRegistry, RuleRegistryBuilder, declare_rule};

declare_rule! {
    /// ## What it does
    /// Reports Tree-sitter recovery `ERROR` nodes.
    ///
    /// ## Why is this bad?
    /// An `ERROR` node means the parser could not reconcile the current token
    /// stream with the grammar and had to recover around unexpected syntax.
    pub static SYNTAX_ERROR_NODE = {
        code: "syntax.error-node",
        summary: "reports unexpected syntax recovered as tree-sitter error nodes",
        default_level: Level::Error,
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
        code: "syntax.missing-node",
        summary: "reports missing syntax recovered by tree-sitter",
        default_level: Level::Error,
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
        code: "include.missing-local-target",
        summary: "reports include directives whose local path does not exist",
        default_level: Level::Error,
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
        code: "include.escapes-allowed-subtree",
        summary: "reports include directives that escape the allowed subtree",
        default_level: Level::Error,
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
        code: "include.cycle",
        summary: "reports explicit include cycles",
        default_level: Level::Error,
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
        code: "include.unsupported-remote-target",
        summary: "reports remote include targets that stay unresolved locally",
        default_level: Level::Warn,
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
        code: "semantic.duplicate-binding",
        summary: "reports duplicate element, deployment, or relationship bindings",
        default_level: Level::Error,
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
        code: "semantic.filtered-view-autolayout-mismatch",
        summary: "reports filtered views built from auto-layout base views",
        default_level: Level::Error,
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
        code: "semantic.dynamic-view-relationship-mismatch",
        summary: "reports dynamic-view steps without a matching declared relationship",
        default_level: Level::Error,
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
        code: "semantic.dynamic-view-scope-redundancy",
        summary: "reports dynamic-view steps that re-add the scoped element",
        default_level: Level::Error,
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
        code: "semantic.invalid-view-element",
        summary: "reports include or animation elements that the current view type cannot show",
        default_level: Level::Error,
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
        code: "semantic.repeated-workspace-section",
        summary: "reports repeated top-level model or views sections",
        default_level: Level::Error,
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
        code: "semantic.unresolved-element-selector",
        summary: "reports unresolved !element selector targets",
        default_level: Level::Error,
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
        code: "semantic.unresolved-reference",
        summary: "reports unresolved identifier references",
        default_level: Level::Error,
    };
}

declare_rule! {
    /// ## What it does
    /// Reports workspace `configuration { scope ... }` values that are shallower than the assembled model depth.
    ///
    /// ## Why is this bad?
    /// Scope mismatches mean the workspace declares one modeling boundary while
    /// still containing deeper elements that upstream validation rejects.
    pub static SEMANTIC_WORKSPACE_SCOPE_MISMATCH = {
        code: "semantic.workspace-scope-mismatch",
        summary: "reports configuration scopes that conflict with model depth",
        default_level: Level::Error,
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
        code: "semantic.ambiguous-reference",
        summary: "reports identifier references with multiple plausible targets",
        default_level: Level::Error,
    };
}

pub fn register_rules(registry: &mut RuleRegistryBuilder) {
    registry.register(&SYNTAX_ERROR_NODE);
    registry.register(&SYNTAX_MISSING_NODE);
    registry.register(&INCLUDE_MISSING_LOCAL_TARGET);
    registry.register(&INCLUDE_ESCAPES_ALLOWED_SUBTREE);
    registry.register(&INCLUDE_CYCLE);
    registry.register(&INCLUDE_UNSUPPORTED_REMOTE_TARGET);
    registry.register(&SEMANTIC_DUPLICATE_BINDING);
    registry.register(&SEMANTIC_FILTERED_VIEW_AUTOLAYOUT_MISMATCH);
    registry.register(&SEMANTIC_DYNAMIC_VIEW_RELATIONSHIP_MISMATCH);
    registry.register(&SEMANTIC_DYNAMIC_VIEW_SCOPE_REDUNDANCY);
    registry.register(&SEMANTIC_INVALID_VIEW_ELEMENT);
    registry.register(&SEMANTIC_REPEATED_WORKSPACE_SECTION);
    registry.register(&SEMANTIC_UNRESOLVED_ELEMENT_SELECTOR);
    registry.register(&SEMANTIC_UNRESOLVED_REFERENCE);
    registry.register(&SEMANTIC_WORKSPACE_SCOPE_MISMATCH);
    registry.register(&SEMANTIC_AMBIGUOUS_REFERENCE);
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
                "semantic.duplicate-binding",
                "semantic.dynamic-view-relationship-mismatch",
                "semantic.dynamic-view-scope-redundancy",
                "semantic.filtered-view-autolayout-mismatch",
                "semantic.invalid-view-element",
                "semantic.repeated-workspace-section",
                "semantic.unresolved-element-selector",
                "semantic.unresolved-reference",
                "semantic.workspace-scope-mismatch",
                "syntax.error-node",
                "syntax.missing-node",
            ]
        );
        assert!(registry.get("syntax.error-node").is_some());
        assert!(registry.get("syntax.missing-node").is_some());
        assert!(registry.get("include.cycle").is_some());
        assert!(registry.get("include.escapes-allowed-subtree").is_some());
        assert!(registry.get("include.missing-local-target").is_some());
        assert!(registry.get("include.unsupported-remote-target").is_some());
        assert!(registry.get("semantic.duplicate-binding").is_some());
        assert!(
            registry
                .get("semantic.filtered-view-autolayout-mismatch")
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
        assert!(registry.get("semantic.invalid-view-element").is_some());
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
