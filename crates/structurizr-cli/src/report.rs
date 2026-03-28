use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;
use structurizr_analysis::{
    DocumentId, DocumentSnapshot, IncludeDiagnostic, IncludeDiagnosticKind, SyntaxDiagnostic,
    SyntaxDiagnosticKind, TextPoint, TextSpan,
};

/// Severity used by the CLI's normalized diagnostic model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// A diagnostic that should fail the command by default.
    Error,
    /// A diagnostic that is shown but does not fail the command by default.
    Warning,
}

impl Severity {
    /// Returns the label used by human-oriented text rendering.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// One-based line and column coordinates for CLI output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PositionView {
    pub line: usize,
    pub column: usize,
}

impl From<TextPoint> for PositionView {
    fn from(point: TextPoint) -> Self {
        Self {
            line: point.row + 1,
            column: point.column + 1,
        }
    }
}

/// Span model shared by diagnostics and dump outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpanView {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start: PositionView,
    pub end: PositionView,
}

impl From<TextSpan> for SpanView {
    fn from(span: TextSpan) -> Self {
        Self {
            start_byte: span.start_byte,
            end_byte: span.end_byte,
            start: span.start_point.into(),
            end: span.end_point.into(),
        }
    }
}

/// One normalized diagnostic emitted by `check` and reused in dump output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticView {
    pub path: String,
    pub severity: Severity,
    pub code: String,
    pub source: String,
    pub message: String,
    pub span: SpanView,
}

impl DiagnosticView {
    /// Builds a normalized syntax diagnostic view.
    #[must_use]
    pub fn syntax(path: String, diagnostic: &SyntaxDiagnostic) -> Self {
        Self {
            path,
            severity: Severity::Error,
            code: syntax_code(diagnostic.kind).to_owned(),
            source: "syntax".to_owned(),
            message: diagnostic.message.clone(),
            span: diagnostic.span.into(),
        }
    }

    /// Builds a normalized include diagnostic view.
    #[must_use]
    pub fn include(path: String, diagnostic: &IncludeDiagnostic) -> Self {
        Self {
            path,
            severity: match diagnostic.kind {
                IncludeDiagnosticKind::UnsupportedRemoteTarget => Severity::Warning,
                IncludeDiagnosticKind::MissingLocalTarget
                | IncludeDiagnosticKind::EscapesAllowedSubtree
                | IncludeDiagnosticKind::IncludeCycle => Severity::Error,
            },
            code: include_code(diagnostic.kind).to_owned(),
            source: "include".to_owned(),
            message: diagnostic.message.clone(),
            span: diagnostic.span.into(),
        }
    }
}

/// Aggregate counts for one `check` run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SummaryView {
    pub documents_checked: usize,
    pub diagnostics: usize,
    pub errors: usize,
    pub warnings: usize,
}

impl SummaryView {
    #[must_use]
    pub fn from_diagnostics(documents_checked: usize, diagnostics: &[DiagnosticView]) -> Self {
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count();

        Self {
            documents_checked,
            diagnostics: diagnostics.len(),
            errors,
            warnings,
        }
    }
}

/// Structured output emitted by `strz check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    pub summary: SummaryView,
    pub diagnostics: Vec<DiagnosticView>,
}

impl CheckReport {
    /// Creates a report and sorts diagnostics into deterministic display order.
    #[must_use]
    pub fn new(documents_checked: usize, mut diagnostics: Vec<DiagnosticView>) -> Self {
        diagnostics.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.span.start.line.cmp(&right.span.start.line))
                .then_with(|| left.span.start.column.cmp(&right.span.start.column))
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.message.cmp(&right.message))
        });

        let summary = SummaryView::from_diagnostics(documents_checked, &diagnostics);

        Self {
            summary,
            diagnostics,
        }
    }

    /// Returns whether the current report should fail the process.
    #[must_use]
    pub const fn should_fail(&self, warnings_as_errors: bool) -> bool {
        self.summary.errors > 0 || (warnings_as_errors && self.summary.warnings > 0)
    }
}

/// One raw include directive as exposed by `dump document`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IncludeDirectiveView {
    pub raw_value: String,
    pub value_kind: String,
    pub container: String,
    pub span: SpanView,
    pub value_span: SpanView,
}

/// One `!identifiers` fact as exposed by `dump document`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdentifierModeView {
    pub mode: String,
    pub raw_value: String,
    pub value_kind: String,
    pub container: String,
    pub span: SpanView,
    pub value_span: SpanView,
}

/// One symbol fact as exposed by `dump document`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolView {
    pub id: usize,
    pub kind: String,
    pub display_name: String,
    pub binding_name: Option<String>,
    pub span: SpanView,
    pub parent: Option<usize>,
    pub syntax_node_kind: String,
}

/// One reference fact as exposed by `dump document`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReferenceView {
    pub kind: String,
    pub raw_text: String,
    pub span: SpanView,
    pub target_hint: String,
    pub container_node_kind: String,
    pub containing_symbol: Option<usize>,
}

/// Structured output emitted by `dump document`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentDump {
    pub path: String,
    pub workspace_entry: bool,
    pub syntax_diagnostics: Vec<DiagnosticView>,
    pub include_directives: Vec<IncludeDirectiveView>,
    pub identifier_modes: Vec<IdentifierModeView>,
    pub symbols: Vec<SymbolView>,
    pub references: Vec<ReferenceView>,
}

/// Summary of one workspace document in `dump workspace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceDocumentView {
    pub path: String,
    pub kind: String,
    pub discovered_by_scan: bool,
    pub syntax_diagnostics: Vec<DiagnosticView>,
    pub include_directive_count: usize,
    pub symbol_count: usize,
    pub reference_count: usize,
}

/// One resolved include entry in `dump workspace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedIncludeView {
    pub document: String,
    pub target_kind: String,
    pub target_text: String,
    pub raw_value: String,
    pub span: SpanView,
    pub value_span: SpanView,
    pub target_location: String,
    pub discovered_documents: Vec<String>,
}

/// Structured output emitted by `dump workspace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceDump {
    pub roots: Vec<String>,
    pub entry_documents: Vec<String>,
    pub documents: Vec<WorkspaceDocumentView>,
    pub includes: Vec<ResolvedIncludeView>,
    pub include_diagnostics: Vec<DiagnosticView>,
}

/// Result of the `dump` command family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpOutput {
    /// `dump document` output.
    Document(DocumentDump),
    /// `dump workspace` output.
    Workspace(WorkspaceDump),
}

/// Returns a canonical current working directory suitable for relative path
/// rendering.
pub fn current_working_directory() -> Result<PathBuf> {
    let cwd = std::env::current_dir()
        .context("while attempting to determine the current working directory")?;
    fs::canonicalize(&cwd).context("while attempting to canonicalize the current working directory")
}

/// Renders a filesystem path relative to the current working directory when
/// possible.
#[must_use]
pub fn display_path(path: &Path, cwd: &Path) -> String {
    match path.strip_prefix(cwd) {
        Ok(relative) if relative.as_os_str().is_empty() => ".".to_owned(),
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

/// Renders a document-backed path for diagnostics and dump output.
#[must_use]
pub fn snapshot_display_path(snapshot: &DocumentSnapshot, cwd: &Path) -> String {
    snapshot.location().map_or_else(
        || document_id_display_path(snapshot.id(), cwd),
        |location| display_path(location.path(), cwd),
    )
}

/// Renders a document identifier as a path when it originated from workspace
/// loading.
#[must_use]
pub fn document_id_display_path(id: &DocumentId, cwd: &Path) -> String {
    display_path(Path::new(id.as_str()), cwd)
}

const fn syntax_code(kind: SyntaxDiagnosticKind) -> &'static str {
    match kind {
        SyntaxDiagnosticKind::ErrorNode => "syntax.error-node",
        SyntaxDiagnosticKind::MissingNode => "syntax.missing-node",
    }
}

const fn include_code(kind: IncludeDiagnosticKind) -> &'static str {
    match kind {
        IncludeDiagnosticKind::MissingLocalTarget => "include.missing-local-target",
        IncludeDiagnosticKind::EscapesAllowedSubtree => "include.escapes-allowed-subtree",
        IncludeDiagnosticKind::IncludeCycle => "include.cycle",
        IncludeDiagnosticKind::UnsupportedRemoteTarget => "include.unsupported-remote-target",
    }
}
