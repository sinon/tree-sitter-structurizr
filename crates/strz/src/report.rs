//! CLI-facing projections of the shared analysis model.
//!
//! The analysis crate owns the canonical diagnostics vocabulary: rule codes,
//! warning/error severity, messages, and source spans. The CLI does not redefine
//! those concepts. Instead, it reshapes them into stable JSON/text views with
//! path-oriented presentation and one-based coordinates.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Serialize, Serializer};
use strz_analysis::{
    DiagnosticSeverity, DocumentId, DocumentLocation, RuledDiagnostic, TextPoint, TextSpan,
};

/// Serialize the analysis-owned severity in the CLI's stable `snake_case` form.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde serialize_with helpers receive field references"
)]
fn serialize_diagnostic_severity<S>(
    severity: &DiagnosticSeverity,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(severity.as_str())
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
    /// Reuse the analysis-owned severity model instead of redefining a parallel
    /// CLI-only enum for the same warning/error distinction.
    #[serde(serialize_with = "serialize_diagnostic_severity")]
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub source: String,
    pub message: String,
    pub span: SpanView,
}

impl DiagnosticView {
    /// Builds a normalized diagnostic view from the shared analysis model.
    #[must_use]
    pub fn from_analysis(path: String, diagnostic: &RuledDiagnostic) -> Self {
        Self {
            path,
            severity: diagnostic.severity(),
            code: diagnostic.code().to_owned(),
            source: diagnostic.source().to_owned(),
            message: diagnostic.message().to_owned(),
            span: diagnostic.span().into(),
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
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
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

/// Whether one format execution checked or wrote documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatModeView {
    /// Report whether formatting would change any document.
    Check,
    /// Rewrite changed documents in place.
    Write,
}

/// Aggregate counts for one `format` run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormatSummaryView {
    pub documents_checked: usize,
    pub changed_documents: usize,
    pub unchanged_documents: usize,
    pub mode: FormatModeView,
}

/// One per-document format result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormatDocumentView {
    pub path: String,
    pub changed: bool,
}

/// Structured output emitted by `strz format`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormatReport {
    pub summary: FormatSummaryView,
    pub documents: Vec<FormatDocumentView>,
}

impl FormatReport {
    /// Creates a deterministic formatter report from one execution mode and result set.
    #[must_use]
    pub fn new(mode: FormatModeView, mut documents: Vec<FormatDocumentView>) -> Self {
        documents.sort_by(|left, right| left.path.cmp(&right.path));
        let changed_documents = documents.iter().filter(|document| document.changed).count();
        let unchanged_documents = documents.len() - changed_documents;

        Self {
            summary: FormatSummaryView {
                documents_checked: documents.len(),
                changed_documents,
                unchanged_documents,
                mode,
            },
            documents,
        }
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

/// Renders one document label for diagnostics and dump output.
///
/// The CLI prefers the concrete filesystem location when one is available.
/// Otherwise it falls back to the raw document id without reinterpreting that
/// identifier as a filesystem path.
#[must_use]
pub fn document_display_path(
    location: Option<&DocumentLocation>,
    fallback_id: &DocumentId,
    cwd: &Path,
) -> String {
    location.map_or_else(
        || document_id_display(fallback_id),
        |location| display_path(location.path(), cwd),
    )
}

/// Renders a document identifier without assuming path semantics.
#[must_use]
pub fn document_id_display(id: &DocumentId) -> String {
    id.as_str().to_owned()
}
