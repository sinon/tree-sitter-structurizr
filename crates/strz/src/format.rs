use std::{fmt::Write as _, fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result, bail};
use strz_analysis::{DocumentInput, WorkspaceLoader};
use strz_format::{FormatError, Formatter};

use crate::{
    cli::FormatArgs,
    report::{
        FormatDocumentView, FormatModeView, FormatReport, current_working_directory,
        snapshot_display_path,
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

    let mut documents = workspace.documents().iter().collect::<Vec<_>>();
    documents.sort_by(|left, right| {
        snapshot_display_path(left.snapshot(), &cwd)
            .cmp(&snapshot_display_path(right.snapshot(), &cwd))
    });

    let mut formatter = Formatter::default();
    let mut formatted_documents = Vec::with_capacity(documents.len());
    let mut blocked_documents = Vec::new();

    for document in documents {
        let path = snapshot_display_path(document.snapshot(), &cwd);
        let location = document
            .snapshot()
            .location()
            .map(|location| location.path().to_path_buf());
        let input = format_input(document);

        match formatter.format_document(input) {
            Ok(document_result) => {
                let changed = document_result.changed();
                let rewritten_source = document_result.into_formatted();
                formatted_documents.push(PendingFormat {
                    path,
                    location,
                    formatted: rewritten_source,
                    changed,
                });
            }
            Err(FormatError::SyntaxErrors { diagnostics }) => {
                blocked_documents.push(BlockedFormat { path, diagnostics });
            }
        }
    }

    if !blocked_documents.is_empty() {
        bail!(blocked_documents_message(&blocked_documents));
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
struct BlockedFormat {
    path: String,
    diagnostics: Vec<strz_analysis::RuledDiagnostic>,
}

fn format_input(document: &strz_analysis::WorkspaceDocument) -> DocumentInput {
    let mut input = DocumentInput::new(
        document.id().clone(),
        document.snapshot().source().to_owned(),
    );
    if let Some(location) = document.snapshot().location() {
        input = input.with_location(location.path());
    }
    input
}

fn joined_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn blocked_documents_message(blocked_documents: &[BlockedFormat]) -> String {
    let mut message = format!(
        "cannot format {} document(s) because syntax recovery is present:",
        blocked_documents.len()
    );

    for blocked in blocked_documents {
        for diagnostic in &blocked.diagnostics {
            let line = diagnostic.span().start_point.row + 1;
            let column = diagnostic.span().start_point.column + 1;
            write!(
                message,
                "\n  {}:{}:{}: {}[{}] {}",
                blocked.path,
                line,
                column,
                diagnostic.severity().as_str(),
                diagnostic.code(),
                diagnostic.message()
            )
            .expect("writing to String should not fail");
        }
    }

    message
}
