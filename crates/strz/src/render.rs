use std::{fmt::Write as _, io::Write};

use anstream::{AutoStream, ColorChoice};
use anstyle::{AnsiColor, Color, Style};
use anyhow::{Context, Error, Result};
use strz_analysis::DiagnosticSeverity;

use crate::{
    cli::{GlobalOptions, OutputFormat},
    report::{CheckReport, DiagnosticView, DocumentDump, DumpOutput, WorkspaceDump},
};

/// Writes `check` output in the requested format.
pub fn write_check(report: &CheckReport, options: &GlobalOptions) -> Result<()> {
    match options.output_format {
        OutputFormat::Text => write_stdout(
            &render_check_text(report, options),
            options.color.to_anstream(),
        ),
        OutputFormat::Json => {
            let rendered = serde_json::to_string_pretty(report)
                .context("while attempting to serialize the check report as JSON")?;
            write_stdout(&(rendered + "\n"), options.color.to_anstream())
        }
    }
}

/// Writes `dump` output in the requested format.
pub fn write_dump(output: &DumpOutput, options: &GlobalOptions) -> Result<()> {
    match options.output_format {
        OutputFormat::Text => write_stdout(
            &render_dump_text(output, options),
            options.color.to_anstream(),
        ),
        OutputFormat::Json => {
            let rendered = match output {
                DumpOutput::Document(document) => serde_json::to_string_pretty(document)
                    .context("while attempting to serialize the document dump as JSON")?,
                DumpOutput::Workspace(workspace) => serde_json::to_string_pretty(workspace)
                    .context("while attempting to serialize the workspace dump as JSON")?,
            };
            write_stdout(&(rendered + "\n"), options.color.to_anstream())
        }
    }
}

/// Writes a runtime error to stderr with a small amount of structure.
pub fn write_error(error: &Error, color_choice: ColorChoice) {
    let mut rendered = String::new();
    let colors = !matches!(color_choice, ColorChoice::Never);
    let label_style = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Red)))
        .bold();

    writeln!(
        rendered,
        "{}: {}",
        styled("error", label_style, colors),
        error
    )
    .expect("writing to String should not fail");

    for cause in error.chain().skip(1) {
        writeln!(rendered, "  caused by: {cause}").expect("writing to String should not fail");
    }

    if let Err(write_error) = write_stderr(&rendered, color_choice) {
        eprintln!("error: {error}");
        eprintln!("error: failed to render stderr output: {write_error}");
    }
}

fn render_check_text(report: &CheckReport, options: &GlobalOptions) -> String {
    let mut output = String::new();
    let colors = !matches!(options.color.to_anstream(), ColorChoice::Never);

    // TODO: Upgrade verbose diagnostics to annotated source snippets if
    // contributor usage shows the current one-line-plus-metadata format is not
    // enough for semantic debugging.
    if report.diagnostics.is_empty() {
        if !options.quiet {
            if options.verbose {
                writeln!(
                    output,
                    "{} {} document(s); no diagnostics found.",
                    styled("checked", heading_style(), colors),
                    report.summary.documents_checked,
                )
                .expect("writing to String should not fail");
            } else {
                writeln!(
                    output,
                    "No diagnostics found in {} document(s).",
                    report.summary.documents_checked,
                )
                .expect("writing to String should not fail");
            }
        }

        return output;
    }

    for diagnostic in &report.diagnostics {
        writeln!(
            output,
            "{}:{}:{}: {}[{}] {}",
            diagnostic.path,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
            styled(
                diagnostic.severity.as_str(),
                severity_style(diagnostic.severity),
                colors,
            ),
            styled(&diagnostic.code, code_style(), colors),
            diagnostic.message,
        )
        .expect("writing to String should not fail");

        if options.verbose {
            writeln!(
                output,
                "  source={} bytes={}..{} end={}:{}",
                diagnostic.source,
                diagnostic.span.start_byte,
                diagnostic.span.end_byte,
                diagnostic.span.end.line,
                diagnostic.span.end.column,
            )
            .expect("writing to String should not fail");
        }
    }

    if options.verbose {
        writeln!(
            output,
            "{} {} error(s), {} warning(s) across {} document(s).",
            styled("summary", heading_style(), colors),
            report.summary.errors,
            report.summary.warnings,
            report.summary.documents_checked,
        )
        .expect("writing to String should not fail");
    }

    output
}

fn render_dump_text(output: &DumpOutput, options: &GlobalOptions) -> String {
    let colors = !matches!(options.color.to_anstream(), ColorChoice::Never);
    match output {
        DumpOutput::Document(document) => render_document_dump(document, colors),
        DumpOutput::Workspace(workspace) => render_workspace_dump(workspace, colors),
    }
}

fn render_document_dump(document: &DocumentDump, colors: bool) -> String {
    let mut output = String::new();
    write_heading(&mut output, "document", colors);
    writeln!(output, "path: {}", document.path).expect("writing to String should not fail");
    writeln!(output, "workspace_entry: {}", document.workspace_entry)
        .expect("writing to String should not fail");
    write_diagnostic_section(
        &mut output,
        "syntax diagnostics",
        &document.syntax_diagnostics,
        colors,
    );

    write_heading(&mut output, "include directives", colors);
    if document.include_directives.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
    } else {
        for directive in &document.include_directives {
            writeln!(
                output,
                "- raw={} value_kind={} container={} at {}:{}",
                directive.raw_value,
                directive.value_kind,
                directive.container,
                directive.span.start.line,
                directive.span.start.column,
            )
            .expect("writing to String should not fail");
        }
    }

    write_heading(&mut output, "identifier modes", colors);
    if document.identifier_modes.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
    } else {
        for fact in &document.identifier_modes {
            writeln!(
                output,
                "- mode={} raw={} container={} at {}:{}",
                fact.mode,
                fact.raw_value,
                fact.container,
                fact.span.start.line,
                fact.span.start.column,
            )
            .expect("writing to String should not fail");
        }
    }

    write_heading(&mut output, "symbols", colors);
    if document.symbols.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
    } else {
        for symbol in &document.symbols {
            writeln!(
                output,
                "- #{} kind={} display_name={} binding_name={} parent={:?} at {}:{}",
                symbol.id,
                symbol.kind,
                symbol.display_name,
                symbol.binding_name.as_deref().unwrap_or("-"),
                symbol.parent,
                symbol.span.start.line,
                symbol.span.start.column,
            )
            .expect("writing to String should not fail");
        }
    }

    write_heading(&mut output, "references", colors);
    if document.references.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
    } else {
        for reference in &document.references {
            writeln!(
                output,
                "- kind={} raw_text={} target_hint={} containing_symbol={:?} at {}:{}",
                reference.kind,
                reference.raw_text,
                reference.target_hint,
                reference.containing_symbol,
                reference.span.start.line,
                reference.span.start.column,
            )
            .expect("writing to String should not fail");
        }
    }

    output
}

fn render_workspace_dump(workspace: &WorkspaceDump, colors: bool) -> String {
    let mut output = String::new();
    write_heading(&mut output, "workspace roots", colors);
    for root in &workspace.roots {
        writeln!(output, "- {root}").expect("writing to String should not fail");
    }

    write_heading(&mut output, "entry documents", colors);
    if workspace.entry_documents.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
    } else {
        for document in &workspace.entry_documents {
            writeln!(output, "- {document}").expect("writing to String should not fail");
        }
    }

    write_heading(&mut output, "documents", colors);
    if workspace.documents.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
    } else {
        for document in &workspace.documents {
            writeln!(
                output,
                "- {} kind={} discovered_by_scan={} syntax_diagnostics={} include_directives={} symbols={} references={}",
                document.path,
                document.kind,
                document.discovered_by_scan,
                document.syntax_diagnostics.len(),
                document.include_directive_count,
                document.symbol_count,
                document.reference_count,
            )
            .expect("writing to String should not fail");
        }
    }

    write_heading(&mut output, "includes", colors);
    if workspace.includes.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
    } else {
        for include in &workspace.includes {
            writeln!(
                output,
                "- {} -> kind={} target={} location={} discovered_documents={}",
                include.document,
                include.target_kind,
                include.target_text,
                include.target_location,
                include.discovered_documents.join(", "),
            )
            .expect("writing to String should not fail");
        }
    }

    write_diagnostic_section(
        &mut output,
        "include diagnostics",
        &workspace.include_diagnostics,
        colors,
    );

    output
}

fn write_diagnostic_section(
    output: &mut String,
    title: &str,
    diagnostics: &[DiagnosticView],
    colors: bool,
) {
    write_heading(output, title, colors);
    if diagnostics.is_empty() {
        writeln!(output, "- none").expect("writing to String should not fail");
        return;
    }

    for diagnostic in diagnostics {
        writeln!(
            output,
            "- {}:{}:{} {}[{}] {}",
            diagnostic.path,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
            styled(
                diagnostic.severity.as_str(),
                severity_style(diagnostic.severity),
                colors,
            ),
            styled(&diagnostic.code, code_style(), colors),
            diagnostic.message,
        )
        .expect("writing to String should not fail");
    }
}

fn write_heading(output: &mut String, title: &str, colors: bool) {
    writeln!(output, "{}:", styled(title, heading_style(), colors))
        .expect("writing to String should not fail");
}

const fn severity_style(severity: DiagnosticSeverity) -> Style {
    let color = match severity {
        DiagnosticSeverity::Error => AnsiColor::Red,
        DiagnosticSeverity::Warning => AnsiColor::Yellow,
    };

    Style::new().fg_color(Some(Color::Ansi(color))).bold()
}

const fn heading_style() -> Style {
    Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Blue)))
        .bold()
}

const fn code_style() -> Style {
    Style::new().dimmed()
}

fn styled(text: &str, style: Style, colors: bool) -> String {
    if colors {
        format!("{}{text}{}", style.render(), style.render_reset())
    } else {
        text.to_owned()
    }
}

fn write_stdout(rendered: &str, color_choice: ColorChoice) -> Result<()> {
    let mut stdout = AutoStream::new(std::io::stdout(), color_choice);
    stdout
        .write_all(rendered.as_bytes())
        .context("while attempting to write CLI output to stdout")?;
    stdout
        .flush()
        .context("while attempting to flush CLI output to stdout")
}

fn write_stderr(rendered: &str, color_choice: ColorChoice) -> Result<()> {
    let mut stderr = AutoStream::new(std::io::stderr(), color_choice);
    stderr
        .write_all(rendered.as_bytes())
        .context("while attempting to write CLI output to stderr")?;
    stderr
        .flush()
        .context("while attempting to flush CLI output to stderr")
}
