use std::path::PathBuf;

use anstream::ColorChoice;
use clap::{Args, Parser, Subcommand, ValueEnum};

/// Root CLI parser for `strz`.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "strz",
    version,
    about = "Run Structurizr checks, contributor dumps, and the LSP server",
    propagate_version = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(subcommand)]
    pub command: Command,
}

/// Shared output and UX controls that apply across subcommands.
#[derive(Debug, Clone, Args)]
pub struct GlobalOptions {
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub output_format: OutputFormat,
    #[arg(long, global = true, value_enum, default_value_t = ColorChoiceArg::Auto)]
    pub color: ColorChoiceArg,
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,
}

/// Top-level commands exposed by the CLI.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Check one or more files or directories for syntax, include, and semantic diagnostics.
    Check(CheckArgs),
    /// Format one or more files or workspaces using the canonical Structurizr layout policy.
    Format(FormatArgs),
    /// Dump analysis-layer facts for a document or workspace.
    Dump(DumpArgs),
    /// Print build metadata for the current `strz` binary.
    Version,
    /// Run the Structurizr LSP server over stdio.
    Server,
}

/// Arguments for the `check` command.
#[derive(Debug, Clone, Args)]
pub struct CheckArgs {
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
    /// Only report parser syntax diagnostics.
    #[arg(long, conflicts_with = "include_only")]
    pub syntax_only: bool,
    /// Only report include-resolution diagnostics.
    #[arg(long, conflicts_with = "syntax_only")]
    pub include_only: bool,
    /// Treat warnings as errors when choosing the process exit code.
    #[arg(long)]
    pub warnings_as_errors: bool,
}

impl CheckArgs {
    /// Returns the requested roots, defaulting to the current directory.
    #[must_use]
    pub fn roots(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        }
    }
}

/// Arguments for the `format` command.
#[derive(Debug, Clone, Args)]
pub struct FormatArgs {
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
    /// Report whether formatting would change any discovered local documents.
    #[arg(long)]
    pub check: bool,
}

impl FormatArgs {
    /// Returns the requested roots, defaulting to the current directory.
    #[must_use]
    pub fn roots(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        }
    }
}

/// Arguments for the `dump` command family.
#[derive(Debug, Clone, Args)]
pub struct DumpArgs {
    #[command(subcommand)]
    pub command: DumpCommand,
}

/// Nested dump commands.
#[derive(Debug, Clone, Subcommand)]
pub enum DumpCommand {
    /// Dump the full analysis snapshot for one document.
    Document(DocumentArgs),
    /// Dump workspace discovery and include-following facts.
    Workspace(WorkspaceArgs),
}

/// Arguments for `dump document`.
#[derive(Debug, Clone, Args)]
pub struct DocumentArgs {
    #[arg(value_name = "PATH")]
    pub path: PathBuf,
}

/// Arguments for `dump workspace`.
#[derive(Debug, Clone, Args)]
pub struct WorkspaceArgs {
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

impl WorkspaceArgs {
    /// Returns the requested roots, defaulting to the current directory.
    #[must_use]
    pub fn roots(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        }
    }
}

/// Output formats shared across check and dump commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-oriented terminal text.
    Text,
    /// Structured JSON intended for tooling and snapshots.
    Json,
}

/// User-facing color policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoiceArg {
    /// Automatically adapt to terminal capabilities.
    Auto,
    /// Always emit color escape codes.
    Always,
    /// Never emit color escape codes.
    Never,
}

impl ColorChoiceArg {
    /// Converts the CLI-facing color choice into `anstream`'s stream setting.
    #[must_use]
    pub const fn to_anstream(self) -> ColorChoice {
        match self {
            Self::Auto => ColorChoice::Auto,
            Self::Always => ColorChoice::Always,
            Self::Never => ColorChoice::Never,
        }
    }
}
