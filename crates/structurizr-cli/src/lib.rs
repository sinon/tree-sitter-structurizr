//! Unified `strz` CLI for contributor analysis commands and the Structurizr LSP.

mod check;
mod cli;
mod dump;
mod observability;
mod render;
mod report;

use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use crate::cli::{Cli, Command};

/// Parses CLI arguments, executes the selected command, and returns a process
/// exit code suitable for `main`.
#[must_use]
pub async fn main() -> ExitCode {
    let cli = Cli::parse();
    let color_choice = cli.global.color.to_anstream();

    match run(&cli).await {
        Ok(exit_code) => exit_code,
        Err(error) => {
            render::write_error(&error, color_choice);
            ExitCode::from(2)
        }
    }
}

async fn run(cli: &Cli) -> Result<ExitCode> {
    observability::init_from_env().context("while attempting to initialize observability")?;

    match &cli.command {
        Command::Check(arguments) => {
            let result =
                check::run(arguments).context("while attempting to execute the check command")?;
            render::write_check(&result.report, &cli.global)
                .context("while attempting to render check output")?;
            Ok(result.exit_code)
        }
        Command::Dump(arguments) => {
            let result =
                dump::run(arguments).context("while attempting to execute the dump command")?;
            render::write_dump(&result, &cli.global)
                .context("while attempting to render dump output")?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Server => {
            structurizr_lsp::serve_stdio().await;
            Ok(ExitCode::SUCCESS)
        }
    }
}
