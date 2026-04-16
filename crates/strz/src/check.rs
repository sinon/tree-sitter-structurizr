use std::{path::Path, process::ExitCode};

use anyhow::{Context, Result};
use strz_analysis::{DocumentId, WorkspaceFacts, WorkspaceLoader};

use crate::{
    cli::CheckArgs,
    report::{
        CheckReport, DiagnosticView, current_working_directory, document_display_path,
        document_id_display,
    },
};

/// Completed result of the `check` command.
#[derive(Debug, Clone)]
pub struct CheckExecution {
    pub report: CheckReport,
    pub exit_code: ExitCode,
}

#[derive(Debug, Clone, Copy)]
enum DiagnosticSelection {
    All,
    SyntaxOnly,
    IncludeOnly,
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

    let selection = match (arguments.syntax_only, arguments.include_only) {
        (true, false) => DiagnosticSelection::SyntaxOnly,
        (false, true) => DiagnosticSelection::IncludeOnly,
        (false, false) => DiagnosticSelection::All,
        (true, true) => unreachable!("clap rejects conflicting check selection flags"),
    };

    let mut diagnostics = Vec::new();
    match selection {
        DiagnosticSelection::SyntaxOnly => {
            for document in workspace.documents() {
                let path =
                    document_display_path(document.snapshot().location(), document.id(), &cwd);
                diagnostics.extend(
                    document
                        .snapshot()
                        .syntax_diagnostics()
                        .iter()
                        .map(|diagnostic| DiagnosticView::from_analysis(path.clone(), diagnostic)),
                );
            }
        }
        DiagnosticSelection::IncludeOnly => {
            for diagnostic in workspace.include_diagnostics() {
                let path = workspace_diagnostic_path(
                    &workspace,
                    diagnostic
                        .document()
                        .expect("workspace include diagnostics should carry documents"),
                    &cwd,
                );
                diagnostics.push(DiagnosticView::from_analysis(path, diagnostic));
            }
        }
        DiagnosticSelection::All => {
            for document in workspace.documents() {
                let path =
                    document_display_path(document.snapshot().location(), document.id(), &cwd);
                diagnostics.extend(
                    document
                        .snapshot()
                        .syntax_diagnostics()
                        .iter()
                        .map(|diagnostic| DiagnosticView::from_analysis(path.clone(), diagnostic)),
                );
            }

            for diagnostic in workspace.include_diagnostics() {
                let path = workspace_diagnostic_path(
                    &workspace,
                    diagnostic
                        .document()
                        .expect("workspace include diagnostics should carry documents"),
                    &cwd,
                );
                diagnostics.push(DiagnosticView::from_analysis(path, diagnostic));
            }

            for diagnostic in workspace.semantic_diagnostics() {
                let path = workspace_diagnostic_path(
                    &workspace,
                    diagnostic
                        .document()
                        .expect("workspace semantic diagnostics should carry documents"),
                    &cwd,
                );
                diagnostics.push(DiagnosticView::from_analysis(path, diagnostic));
            }
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
        || document_id_display(document),
        |document| document_display_path(document.snapshot().location(), document.id(), cwd),
    )
}
