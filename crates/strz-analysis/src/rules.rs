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

pub fn register_rules(registry: &mut RuleRegistryBuilder) {
    registry.register(&SYNTAX_ERROR_NODE);
    registry.register(&SYNTAX_MISSING_NODE);
    registry.register(&INCLUDE_MISSING_LOCAL_TARGET);
    registry.register(&INCLUDE_ESCAPES_ALLOWED_SUBTREE);
    registry.register(&INCLUDE_CYCLE);
    registry.register(&INCLUDE_UNSUPPORTED_REMOTE_TARGET);
    registry.register(&SEMANTIC_DUPLICATE_BINDING);
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
