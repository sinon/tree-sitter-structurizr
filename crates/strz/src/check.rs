use std::{path::Path, process::ExitCode};

use anyhow::{Context, Result};
use strz_analysis::{DocumentId, WorkspaceFacts, WorkspaceLoader};

use crate::{
    cli::CheckArgs,
    report::{
        current_working_directory, document_id_display_path, snapshot_display_path, CheckReport,
        DiagnosticView,
    },
};

/// Completed result of the `check` command.
#[derive(Debug, Clone)]
pub struct CheckExecution {
    pub report: CheckReport,
    pub exit_code: ExitCode,
}

/// Runs the `check` command against one or more files or directories.
pub fn run(arguments: &CheckArgs) -> Result<CheckExecution> {
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

    let mut diagnostics = Vec::new();

    if !arguments.include_only {
        for document in workspace.documents() {
            let path = snapshot_display_path(document.snapshot(), &cwd);
            diagnostics.extend(
                document
                    .snapshot()
                    .syntax_diagnostics()
                    .iter()
                    .map(|diagnostic| DiagnosticView::syntax(path.clone(), diagnostic)),
            );
        }
    }

    if !arguments.syntax_only {
        for diagnostic in workspace.include_diagnostics() {
            let path = workspace_diagnostic_path(&workspace, &diagnostic.document, &cwd);
            diagnostics.push(DiagnosticView::include(path, diagnostic));
        }
    }

    if !arguments.syntax_only && !arguments.include_only {
        for diagnostic in workspace.semantic_diagnostics() {
            let path = workspace_diagnostic_path(&workspace, &diagnostic.document, &cwd);
            diagnostics.push(DiagnosticView::semantic(path, diagnostic));
        }
    }

    let report = CheckReport::new(workspace.documents().len(), diagnostics);
    let exit_code = if report.should_fail(arguments.warnings_as_errors) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    };

    Ok(CheckExecution { report, exit_code })
}

fn joined_paths(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn workspace_diagnostic_path(
    workspace: &WorkspaceFacts,
    document: &DocumentId,
    cwd: &Path,
) -> String {
    workspace.document(document).map_or_else(
        || document_id_display_path(document, cwd),
        |document| snapshot_display_path(document.snapshot(), cwd),
    )
}
