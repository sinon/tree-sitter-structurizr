//! Transport-agnostic diagnostics derived from parsing and workspace discovery.

use std::fmt;

use crate::{
    rule::{Level, RuleMetadata, RuleRegistry},
    rules,
    snapshot::DocumentId,
    span::TextSpan,
};

/// Severity carried by transport-agnostic diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// A diagnostic that should fail normal validation flows.
    Error,
    /// A diagnostic that should be shown without failing by default.
    Warning,
}

impl DiagnosticSeverity {
    /// Maps one declared rule level to the corresponding transport severity.
    #[must_use]
    pub const fn from_level(level: Level) -> Self {
        match level {
            Level::Warn => Self::Warning,
            Level::Error => Self::Error,
        }
    }
}

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

/// Categorizes the syntax problem Tree-sitter reported for a source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxDiagnosticKind {
    /// Tree-sitter produced an `ERROR` node for unexpected syntax.
    ErrorNode,
    /// Tree-sitter synthesized a `MISSING` node to recover from absent syntax.
    MissingNode,
}

impl SyntaxDiagnosticKind {
    /// Returns the declared rule metadata for this syntax problem.
    #[must_use]
    pub const fn rule(self) -> &'static RuleMetadata {
        match self {
            Self::ErrorNode => &rules::SYNTAX_ERROR_NODE,
            Self::MissingNode => &rules::SYNTAX_MISSING_NODE,
        }
    }

    /// Returns the stable diagnostic code for this syntax problem.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.rule().code()
    }

    /// Returns the severity for this syntax problem.
    #[must_use]
    pub const fn severity(self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_level(self.rule().default_level())
    }
}

/// Describes a syntax problem extracted from the parse tree.
#[derive(Clone, PartialEq, Eq)]
pub struct SyntaxDiagnostic {
    /// The parser-reported category of syntax problem.
    pub kind: SyntaxDiagnosticKind,
    /// Human-readable summary of the syntax problem.
    pub message: String,
    /// Byte and point range covered by the diagnostic.
    pub span: TextSpan,
    /// Secondary source spans that provide extra context.
    pub annotations: Vec<Annotation>,
}

impl fmt::Debug for SyntaxDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("SyntaxDiagnostic");
        debug
            .field("kind", &self.kind)
            .field("message", &self.message)
            .field("span", &self.span);
        if !self.annotations.is_empty() {
            debug.field("annotations", &self.annotations);
        }
        debug.finish()
    }
}

impl SyntaxDiagnostic {
    /// Returns the declared rule metadata for this syntax problem.
    #[must_use]
    pub const fn rule(&self) -> &'static RuleMetadata {
        self.kind.rule()
    }

    /// Returns the stable diagnostic code for this syntax problem.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.rule().code()
    }

    /// Returns the severity for this syntax problem.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_level(self.rule().default_level())
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

    pub(crate) fn unexpected_syntax(span: TextSpan) -> Self {
        Self {
            kind: SyntaxDiagnosticKind::ErrorNode,
            message: "unexpected syntax".to_owned(),
            span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn missing_node(kind: &str, span: TextSpan) -> Self {
        Self {
            kind: SyntaxDiagnosticKind::MissingNode,
            message: format!("missing {kind}"),
            span,
            annotations: Vec::new(),
        }
    }
}

/// Categorizes include-resolution problems discovered while loading a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncludeDiagnosticKind {
    /// The include target did not exist on disk.
    MissingLocalTarget,
    /// The include target escaped or otherwise violated local path policy.
    EscapesAllowedSubtree,
    /// The include target participates in an explicit include cycle.
    IncludeCycle,
    /// The include target is remote and therefore left unresolved in the MVP.
    UnsupportedRemoteTarget,
}

impl IncludeDiagnosticKind {
    /// Returns the declared rule metadata for this include problem.
    #[must_use]
    pub const fn rule(self) -> &'static RuleMetadata {
        match self {
            Self::MissingLocalTarget => &rules::INCLUDE_MISSING_LOCAL_TARGET,
            Self::EscapesAllowedSubtree => &rules::INCLUDE_ESCAPES_ALLOWED_SUBTREE,
            Self::IncludeCycle => &rules::INCLUDE_CYCLE,
            Self::UnsupportedRemoteTarget => &rules::INCLUDE_UNSUPPORTED_REMOTE_TARGET,
        }
    }

    /// Returns the stable diagnostic code for this include problem.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.rule().code()
    }

    /// Returns the severity for this include problem.
    #[must_use]
    pub const fn severity(self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_level(self.rule().default_level())
    }
}

/// Describes one include-resolution problem attached to a directive site.
#[derive(Clone, PartialEq, Eq)]
pub struct IncludeDiagnostic {
    /// The including document that should surface this diagnostic.
    pub document: DocumentId,
    /// The include-resolution category.
    pub kind: IncludeDiagnosticKind,
    /// Human-readable summary of the include problem.
    pub message: String,
    /// Normalized include target text with surrounding quotes stripped.
    pub target_text: String,
    /// Span of the full include directive in the including document.
    pub span: TextSpan,
    /// Span of the directive value node in the including document.
    pub value_span: TextSpan,
    /// Secondary source spans that provide extra context.
    pub annotations: Vec<Annotation>,
}

impl fmt::Debug for IncludeDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("IncludeDiagnostic");
        debug
            .field("document", &self.document)
            .field("kind", &self.kind)
            .field("message", &self.message)
            .field("target_text", &self.target_text)
            .field("span", &self.span)
            .field("value_span", &self.value_span);
        if !self.annotations.is_empty() {
            debug.field("annotations", &self.annotations);
        }
        debug.finish()
    }
}

impl IncludeDiagnostic {
    /// Returns the declared rule metadata for this include problem.
    #[must_use]
    pub const fn rule(&self) -> &'static RuleMetadata {
        self.kind.rule()
    }

    /// Returns the stable diagnostic code for this include problem.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.rule().code()
    }

    /// Returns the severity for this include problem.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_level(self.rule().default_level())
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

    pub(crate) fn missing_local_target(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::MissingLocalTarget,
            message: format!("included path does not exist: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn escapes_allowed_subtree(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::EscapesAllowedSubtree,
            message: format!("included path escapes the allowed subtree: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn include_cycle(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::IncludeCycle,
            message: format!("include cycle detected while following: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn unsupported_remote_target(
        document: &DocumentId,
        target_text: &str,
        span: TextSpan,
        value_span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: IncludeDiagnosticKind::UnsupportedRemoteTarget,
            message: format!("remote includes are not resolved in the MVP: {target_text}"),
            target_text: target_text.to_owned(),
            span,
            value_span,
            annotations: Vec::new(),
        }
    }
}

/// Categorizes bounded semantic diagnostics derived from workspace indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticDiagnosticKind {
    /// More than one definition claimed the same canonical binding key.
    DuplicateBinding,
    /// One DSL definition contained more than one top-level `model` or `views` section.
    RepeatedWorkspaceSection,
    /// A supported `!element` selector target resolved to no known target.
    UnresolvedElementSelector,
    /// A supported identifier reference resolved to no known target.
    UnresolvedReference,
    /// A declared workspace scope conflicts with the assembled model depth.
    WorkspaceScopeMismatch,
    /// A supported identifier reference could not be resolved confidently.
    AmbiguousReference,
}

impl SemanticDiagnosticKind {
    /// Returns the declared rule metadata for this semantic problem.
    #[must_use]
    pub const fn rule(self) -> &'static RuleMetadata {
        match self {
            Self::DuplicateBinding => &rules::SEMANTIC_DUPLICATE_BINDING,
            Self::RepeatedWorkspaceSection => &rules::SEMANTIC_REPEATED_WORKSPACE_SECTION,
            Self::UnresolvedElementSelector => &rules::SEMANTIC_UNRESOLVED_ELEMENT_SELECTOR,
            Self::UnresolvedReference => &rules::SEMANTIC_UNRESOLVED_REFERENCE,
            Self::WorkspaceScopeMismatch => &rules::SEMANTIC_WORKSPACE_SCOPE_MISMATCH,
            Self::AmbiguousReference => &rules::SEMANTIC_AMBIGUOUS_REFERENCE,
        }
    }

    /// Returns the stable diagnostic code for this semantic rule.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.rule().code()
    }

    /// Returns the default severity for this semantic rule.
    #[must_use]
    pub const fn severity(self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_level(self.rule().default_level())
    }
}

/// Returns the registry of currently declared diagnostic rules.
#[must_use]
pub fn diagnostic_rule_registry() -> &'static RuleRegistry {
    rules::diagnostic_rule_registry()
}

/// Describes one semantic problem attached to a definition or reference site.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticDiagnostic {
    /// The document that should surface this diagnostic.
    pub document: DocumentId,
    /// The semantic-diagnostic category.
    pub kind: SemanticDiagnosticKind,
    /// Human-readable summary of the semantic problem.
    pub message: String,
    /// Span of the affected symbol or reference.
    pub span: TextSpan,
    /// Secondary source spans that provide extra context.
    pub annotations: Vec<Annotation>,
}

impl fmt::Debug for SemanticDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("SemanticDiagnostic");
        debug
            .field("document", &self.document)
            .field("kind", &self.kind)
            .field("message", &self.message)
            .field("span", &self.span);
        if !self.annotations.is_empty() {
            debug.field("annotations", &self.annotations);
        }
        debug.finish()
    }
}

impl SemanticDiagnostic {
    /// Returns the declared rule metadata for this semantic problem.
    #[must_use]
    pub const fn rule(&self) -> &'static RuleMetadata {
        self.kind.rule()
    }

    /// Returns the stable diagnostic code for this semantic rule.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.rule().code()
    }

    /// Returns the severity for this semantic rule.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        DiagnosticSeverity::from_level(self.rule().default_level())
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

    pub(crate) fn duplicate_binding(
        document: &DocumentId,
        binding_kind: &str,
        key: &str,
        span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::DuplicateBinding,
            message: format!("duplicate {binding_kind} binding: {key}"),
            span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn repeated_workspace_section(
        document: &DocumentId,
        section_name: &str,
        span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::RepeatedWorkspaceSection,
            message: format!(
                "multiple {section_name} sections are not permitted in a DSL definition"
            ),
            span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn unresolved_element_selector(
        document: &DocumentId,
        raw_text: &str,
        span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::UnresolvedElementSelector,
            message: format!("unresolved !element selector target: {raw_text}"),
            span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn unresolved_reference(
        document: &DocumentId,
        raw_text: &str,
        span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::UnresolvedReference,
            message: format!("unresolved identifier reference: {raw_text}"),
            span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn workspace_scope_mismatch(
        document: &DocumentId,
        message: impl Into<String>,
        span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::WorkspaceScopeMismatch,
            message: message.into(),
            span,
            annotations: Vec::new(),
        }
    }

    pub(crate) fn ambiguous_reference(
        document: &DocumentId,
        raw_text: &str,
        span: TextSpan,
    ) -> Self {
        Self {
            document: document.clone(),
            kind: SemanticDiagnosticKind::AmbiguousReference,
            message: format!("ambiguous identifier reference: {raw_text}"),
            span,
            annotations: Vec::new(),
        }
    }
}
