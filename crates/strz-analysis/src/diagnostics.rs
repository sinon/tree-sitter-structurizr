//! Transport-agnostic diagnostics derived from parsing and workspace discovery.
//!
//! The important architectural split is:
//!
//! 1. [`crate::rule`] owns rule identity and metadata.
//! 2. [`Diagnostic`] owns emitted problem data and nothing else.
//! 3. [`RuledDiagnostic`] pairs one emitted payload with one declared rule so CLI
//!    and LSP consumers can still render codes and severity without teaching the
//!    payload about the registry.
//!
//! That keeps the emitted diagnostic record intentionally boring. It knows the
//! user-facing message, the relevant spans, and some optional context fields, but
//! it does not decide what class of problem it is. That decision belongs to the
//! rule layer.

use std::fmt;

use crate::{
    rule::{RuleId, RuleMetadata, RuleRegistry},
    rules,
    snapshot::DocumentId,
    span::TextSpan,
};

// =============================================================================
// Shared emitted-diagnostic building blocks
// =============================================================================

/// Secondary source context attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Annotation {
    /// Optional document identity when the annotation points outside the primary
    /// diagnostic document.
    pub document: Option<DocumentId>,
    /// Byte and point range covered by the related source span.
    pub span: TextSpan,
    /// Human-readable explanation for why the related span matters.
    pub message: Option<String>,
}

impl Annotation {
    /// Creates a secondary annotation in the same document as the primary span.
    #[must_use]
    pub const fn secondary(span: TextSpan) -> Self {
        Self {
            document: None,
            span,
            message: None,
        }
    }

    /// Associates the annotation with a different document.
    #[must_use]
    pub fn in_document(mut self, document: &DocumentId) -> Self {
        self.document = Some(document.clone());
        self
    }

    /// Attaches a terse explanatory message to the annotation.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

// =============================================================================
// Pure emitted diagnostic payload
// =============================================================================

/// One emitted problem plus the data needed to explain it to a user.
///
/// This type intentionally does not know which rule produced it. That keeps the
/// payload reusable across syntax recovery, include resolution, and semantic
/// analysis without requiring a growing enum or registry lookup table here.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    /// Human-readable explanation of the concrete problem at this site.
    pub message: String,
    /// Primary span that transport consumers should highlight.
    pub span: TextSpan,
    /// Additional related spans that explain the problem more fully.
    pub annotations: Vec<Annotation>,
    /// Optional owning document for workspace-scoped diagnostics.
    pub document: Option<DocumentId>,
    /// Optional user-facing target text associated with this problem.
    pub target_text: Option<String>,
    /// Optional value-specific span when the whole statement span is too broad.
    pub value_span: Option<TextSpan>,
}

impl fmt::Debug for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("Diagnostic");
        debug
            .field("message", &self.message)
            .field("span", &self.span);
        if let Some(document) = &self.document {
            debug.field("document", document);
        }
        if let Some(target_text) = &self.target_text {
            debug.field("target_text", target_text);
        }
        if let Some(value_span) = self.value_span {
            debug.field("value_span", &value_span);
        }
        if !self.annotations.is_empty() {
            debug.field("annotations", &self.annotations);
        }
        debug.finish()
    }
}

impl Diagnostic {
    fn new(message: impl Into<String>, span: TextSpan) -> Self {
        Self {
            message: message.into(),
            span,
            annotations: Vec::new(),
            document: None,
            target_text: None,
            value_span: None,
        }
    }

    fn in_document(mut self, document: &DocumentId) -> Self {
        self.document = Some(document.clone());
        self
    }

    fn with_target_text(mut self, target_text: impl Into<String>) -> Self {
        self.target_text = Some(target_text.into());
        self
    }

    const fn with_value_span(mut self, value_span: TextSpan) -> Self {
        self.value_span = Some(value_span);
        self
    }

    /// Returns any secondary source context attached to this diagnostic.
    #[must_use]
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    /// Attaches one secondary annotation to this diagnostic.
    pub fn annotate(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }

    /// Returns the owning document when this diagnostic is workspace-scoped.
    #[must_use]
    pub const fn document(&self) -> Option<&DocumentId> {
        self.document.as_ref()
    }

    /// Returns the user-facing target text attached to this diagnostic, if any.
    #[must_use]
    pub fn target_text(&self) -> Option<&str> {
        self.target_text.as_deref()
    }

    /// Returns the narrower value span attached to this diagnostic, if any.
    #[must_use]
    pub const fn value_span(&self) -> Option<TextSpan> {
        self.value_span
    }
}

// =============================================================================
// Rule-tagged emitted diagnostics
// =============================================================================

/// One emitted diagnostic payload paired with the rule that produced it.
///
/// This is the type that analysis produces and transport consumers render. The
/// payload remains generic, while the envelope carries the stable rule identity
/// needed for code, severity, and documentation lookup.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuledDiagnostic {
    /// Declared rule that classified the emitted problem.
    pub rule: RuleId,
    /// Concrete payload observed in the analyzed source.
    pub diagnostic: Diagnostic,
}

impl fmt::Debug for RuledDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuledDiagnostic")
            .field("rule", &self.rule)
            .field("diagnostic", &self.diagnostic)
            .finish()
    }
}
impl RuledDiagnostic {
    const fn new(rule: RuleId, diagnostic: Diagnostic) -> Self {
        Self { rule, diagnostic }
    }

    /// Returns the declared rule metadata for this emitted diagnostic.
    #[must_use]
    pub const fn rule(&self) -> &'static RuleMetadata {
        self.rule.metadata()
    }

    /// Returns the stable diagnostic code for this emitted diagnostic.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.rule.code()
    }

    /// Returns the normalized severity for this emitted diagnostic.
    #[must_use]
    pub const fn severity(&self) -> crate::rule::DiagnosticSeverity {
        self.rule.severity()
    }

    /// Returns the broad analysis stage that produced this diagnostic.
    #[must_use]
    pub const fn source(&self) -> &'static str {
        self.rule.source()
    }

    /// Returns the human-readable message for this diagnostic.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.diagnostic.message
    }

    /// Returns the primary highlight span.
    #[must_use]
    pub const fn span(&self) -> TextSpan {
        self.diagnostic.span
    }

    /// Returns any secondary source context attached to this diagnostic.
    #[must_use]
    pub fn annotations(&self) -> &[Annotation] {
        self.diagnostic.annotations()
    }

    /// Attaches one secondary annotation to this diagnostic.
    pub fn annotate(&mut self, annotation: Annotation) {
        self.diagnostic.annotate(annotation);
    }

    /// Returns the owning document when this diagnostic is workspace-scoped.
    #[must_use]
    pub const fn document(&self) -> Option<&DocumentId> {
        self.diagnostic.document()
    }

    /// Returns the user-facing target text attached to this diagnostic, if any.
    #[must_use]
    pub fn target_text(&self) -> Option<&str> {
        self.diagnostic.target_text()
    }

    /// Returns the narrower value span attached to this diagnostic, if any.
    #[must_use]
    pub const fn value_span(&self) -> Option<TextSpan> {
        self.diagnostic.value_span()
    }

    /// Returns whether this diagnostic was emitted for the given rule.
    #[must_use]
    pub fn is_rule(&self, rule: RuleId) -> bool {
        self.rule == rule
    }

    // -------------------------------------------------------------------------
    // Syntax constructors
    // -------------------------------------------------------------------------

    pub(crate) fn unexpected_syntax(span: TextSpan) -> Self {
        Self::new(
            rules::SYNTAX_ERROR_NODE.id(),
            Diagnostic::new("unexpected syntax", span),
        )
    }

    pub(crate) fn missing_node(kind: &str, span: TextSpan) -> Self {
        Self::new(
            rules::SYNTAX_MISSING_NODE.id(),
            Diagnostic::new(format!("missing {kind}"), span),
        )
    }

    // -------------------------------------------------------------------------
    // Include constructors
    // -------------------------------------------------------------------------

    pub(crate) fn missing_local_target(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self::new(
            rules::INCLUDE_MISSING_LOCAL_TARGET.id(),
            Diagnostic::new(format!("included path does not exist: {target_text}"), span)
                .in_document(document)
                .with_target_text(target_text)
                .with_value_span(value_span),
        )
    }

    pub(crate) fn escapes_allowed_subtree(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self::new(
            rules::INCLUDE_ESCAPES_ALLOWED_SUBTREE.id(),
            Diagnostic::new(
                format!("included path escapes the allowed subtree: {target_text}"),
                span,
            )
            .in_document(document)
            .with_target_text(target_text)
            .with_value_span(value_span),
        )
    }

    pub(crate) fn include_cycle(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self::new(
            rules::INCLUDE_CYCLE.id(),
            Diagnostic::new(
                format!("include cycle detected while following: {target_text}"),
                span,
            )
            .in_document(document)
            .with_target_text(target_text)
            .with_value_span(value_span),
        )
    }

    pub(crate) fn unsupported_remote_target(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self::new(
            rules::INCLUDE_UNSUPPORTED_REMOTE_TARGET.id(),
            Diagnostic::new(
                format!("remote includes are not resolved in the MVP: {target_text}"),
                span,
            )
            .in_document(document)
            .with_target_text(target_text)
            .with_value_span(value_span),
        )
    }

    // -------------------------------------------------------------------------
    // Semantic constructors
    // -------------------------------------------------------------------------

    pub(crate) fn duplicate_binding(
        document: &DocumentId,
        binding_kind: &str,
        key: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_DUPLICATE_BINDING.id(),
            Diagnostic::new(format!("duplicate {binding_kind} binding: {key}"), span)
                .in_document(document)
                .with_target_text(key),
        )
    }

    pub(crate) fn repeated_workspace_section(
        document: &DocumentId,
        section_name: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_REPEATED_WORKSPACE_SECTION.id(),
            Diagnostic::new(
                format!("multiple {section_name} sections are not permitted in a DSL definition"),
                span,
            )
            .in_document(document),
        )
    }

    pub(crate) fn unresolved_element_selector(
        document: &DocumentId,
        raw_text: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_UNRESOLVED_ELEMENT_SELECTOR.id(),
            Diagnostic::new(
                format!("unresolved !element selector target: {raw_text}"),
                span,
            )
            .in_document(document)
            .with_target_text(raw_text),
        )
    }

    pub(crate) fn unresolved_reference(
        document: &DocumentId,
        raw_text: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_UNRESOLVED_REFERENCE.id(),
            Diagnostic::new(format!("unresolved identifier reference: {raw_text}"), span)
                .in_document(document)
                .with_target_text(raw_text),
        )
    }

    pub(crate) fn workspace_scope_mismatch(
        document: &DocumentId,
        message: impl Into<String>,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_WORKSPACE_SCOPE_MISMATCH.id(),
            Diagnostic::new(message, span).in_document(document),
        )
    }

    pub(crate) fn filtered_view_autolayout_mismatch(
        document: &DocumentId,
        base_key: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_FILTERED_VIEW_AUTOLAYOUT_MISMATCH.id(),
            Diagnostic::new(
                format!(
                    "The view \"{base_key}\" has automatic layout enabled - this is not supported for filtered views"
                ),
                span,
            )
            .in_document(document)
            .with_target_text(base_key),
        )
    }

    pub(crate) fn deployment_parent_child_relationship(
        document: &DocumentId,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_DEPLOYMENT_PARENT_CHILD_RELATIONSHIP.id(),
            Diagnostic::new(
                "Relationships cannot be added between parents and children",
                span,
            )
            .in_document(document),
        )
    }

    pub(crate) fn dynamic_view_relationship_mismatch(
        document: &DocumentId,
        source_name: &str,
        destination_name: &str,
        technology: Option<&str>,
        span: TextSpan,
    ) -> Self {
        let message = technology.map_or_else(
            || {
                format!(
                    "A relationship between {source_name} and {destination_name} does not exist in model."
                )
            },
            |technology| {
                format!(
                    "A relationship between {source_name} and {destination_name} with technology {technology} does not exist in model."
                )
            },
        );

        Self::new(
            rules::SEMANTIC_DYNAMIC_VIEW_RELATIONSHIP_MISMATCH.id(),
            Diagnostic::new(message, span).in_document(document),
        )
    }

    pub(crate) fn dynamic_view_scope_redundancy(
        document: &DocumentId,
        scope_name: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_DYNAMIC_VIEW_SCOPE_REDUNDANCY.id(),
            Diagnostic::new(
                format!("{scope_name} is already the scope of this view and cannot be added to it"),
                span,
            )
            .in_document(document)
            .with_target_text(scope_name),
        )
    }

    pub(crate) fn invalid_view_element(
        document: &DocumentId,
        raw_text: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_INVALID_VIEW_ELEMENT.id(),
            Diagnostic::new(
                format!("The element \"{raw_text}\" can not be added to this type of view"),
                span,
            )
            .in_document(document)
            .with_target_text(raw_text),
        )
    }

    pub(crate) fn invalid_documentation_path(
        document: &DocumentId,
        message: impl Into<String>,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_INVALID_DOCUMENTATION_PATH.id(),
            Diagnostic::new(message, span).in_document(document),
        )
    }

    pub(crate) fn invalid_image_source(
        document: &DocumentId,
        message: impl Into<String>,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_INVALID_IMAGE_SOURCE.id(),
            Diagnostic::new(message, span).in_document(document),
        )
    }

    pub(crate) fn missing_image_renderer_property(
        document: &DocumentId,
        property_name: &str,
        service_name: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_MISSING_IMAGE_RENDERER_PROPERTY.id(),
            Diagnostic::new(
                // Keep the guidance text aligned with Structurizr's importer
                // exceptions so validation and upstream runtime failures read the
                // same way.
                format!(
                    "Please define a view/viewset property named {property_name} to specify your {service_name} server"
                ),
                span,
            )
            .in_document(document),
        )
    }

    pub(crate) fn ambiguous_reference(
        document: &DocumentId,
        raw_text: &str,
        span: TextSpan,
    ) -> Self {
        Self::new(
            rules::SEMANTIC_AMBIGUOUS_REFERENCE.id(),
            Diagnostic::new(format!("ambiguous identifier reference: {raw_text}"), span)
                .in_document(document)
                .with_target_text(raw_text),
        )
    }
}

/// Returns the registry of currently declared diagnostic rules.
#[must_use]
pub fn diagnostic_rule_registry() -> &'static RuleRegistry {
    rules::diagnostic_rule_registry()
}
