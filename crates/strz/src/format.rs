use std::{fmt, fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result};
use strz_analysis::{DocumentInput, WorkspaceLoader};
use strz_format::{FormatError, Formatter};

use crate::{
    cli::FormatArgs,
    report::{
        FormatDocumentView, FormatModeView, FormatReport, current_working_directory,
        document_display_path,
    },
};

/// Completed result of one `format` command execution.
#[derive(Debug, Clone)]
pub struct FormatExecution {
    pub report: FormatReport,
    pub exit_code: ExitCode,
}

/// Runs the `format` command against one or more files or directories.
pub fn run(arguments: &FormatArgs) -> Result<FormatExecution> {
    let cwd = current_working_directory()
        .context("while attempting to determine the CLI display root")?;
    let roots = arguments.roots();

    let mut loader = WorkspaceLoader::new();
    let workspace = loader.load_paths(&roots).with_context(|| {
        format!(
            "while attempting to load workspace roots: {}",
            joined_paths(&roots)
        )
    })?;

    let mut documents = workspace
        .documents()
        .iter()
        .map(|document| prepare_format_target(document, &cwd))
        .collect::<Vec<_>>();
    documents.sort_by(|left, right| left.path.cmp(&right.path));

    let mut formatter = Formatter::default();
    let mut formatted_documents = Vec::with_capacity(documents.len());
    let mut blocked_documents = Vec::new();

    for document in documents {
        match formatter.format_document(document.input) {
            Ok(document_result) => {
                let changed = document_result.changed();
                let rewritten_source = document_result.into_formatted();
                formatted_documents.push(PendingFormat {
                    path: document.path,
                    location: document.location,
                    formatted: rewritten_source,
                    changed,
                });
            }
            Err(FormatError::SyntaxErrors { diagnostics }) => {
                blocked_documents.push(BlockedFormat {
                    path: document.path,
                    diagnostics,
                });
            }
        }
    }

    if !blocked_documents.is_empty() {
        return Err(BlockedFormatsError::new(blocked_documents).into());
    }

    if !arguments.check {
        for document in &formatted_documents {
            if !document.changed {
                continue;
            }
            let path = document.location.as_ref().with_context(|| {
                format!(
                    "while attempting to locate the on-disk path for {}",
                    document.path
                )
            })?;
            fs::write(path, &document.formatted)
                .with_context(|| format!("while attempting to write {}", path.display()))?;
        }
    }

    let report = FormatReport::new(
        if arguments.check {
            FormatModeView::Check
        } else {
            FormatModeView::Write
        },
        formatted_documents
            .into_iter()
            .map(|document| FormatDocumentView {
                path: document.path,
                changed: document.changed,
            })
            .collect(),
    );
    let exit_code = if arguments.check && report.summary.changed_documents > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    };

    Ok(FormatExecution { report, exit_code })
}

#[derive(Debug, Clone)]
struct PendingFormat {
    path: String,
    location: Option<PathBuf>,
    formatted: String,
    changed: bool,
}

#[derive(Debug, Clone)]
struct PreparedFormatTarget {
    path: String,
    location: Option<PathBuf>,
    input: DocumentInput,
}

#[derive(Debug, Clone)]
struct BlockedFormat {
    path: String,
    diagnostics: Vec<strz_analysis::RuledDiagnostic>,
}

#[derive(Debug, Clone)]
struct BlockedFormatsError {
    blocked_documents: Vec<BlockedFormat>,
}

impl BlockedFormatsError {
    const fn new(blocked_documents: Vec<BlockedFormat>) -> Self {
        Self { blocked_documents }
    }
}

impl fmt::Display for BlockedFormatsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot format {} document(s) because syntax errors are present:",
            self.blocked_documents.len()
        )?;
        for blocked in &self.blocked_documents {
            write!(formatter, "{blocked}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BlockedFormatsError {}

impl fmt::Display for BlockedFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.diagnostics.is_empty() {
            return write!(formatter, "\n  {}: syntax errors are present", self.path);
        }
        for diagnostic in &self.diagnostics {
            let line = diagnostic.span().start_point.row + 1;
            let column = diagnostic.span().start_point.column + 1;
            write!(
                formatter,
                "\n  {}:{}:{}: {}[{}] {}",
                self.path,
                line,
                column,
                diagnostic.severity().as_str(),
                diagnostic.code(),
                diagnostic.message()
            )?;
        }
        Ok(())
    }
}

fn prepare_format_target(
    document: &strz_analysis::WorkspaceDocument,
    cwd: &std::path::Path,
) -> PreparedFormatTarget {
    let mut input = DocumentInput::new(
        document.id().clone(),
        document.snapshot().source().to_owned(),
    );
    if let Some(location) = document.snapshot().location() {
        input = input.with_location(location.path());
    }
    PreparedFormatTarget {
        path: document_display_path(document.snapshot().location(), document.id(), cwd),
        location: document
            .snapshot()
            .location()
            .map(|location| location.path().to_path_buf()),
        input,
    }
}

fn joined_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
