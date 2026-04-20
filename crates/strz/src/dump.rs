use std::fs;

use anyhow::{Context, Result};
use strz_analysis::{
    DirectiveContainer, DirectiveValueKind, DocumentAnalyzer, DocumentInput, IdentifierMode,
    ReferenceKind, ReferenceTargetHint, ResolvedInclude, RuledDiagnostic, SymbolKind,
    WorkspaceDocument, WorkspaceDocumentKind, WorkspaceFacts, WorkspaceIncludeTarget,
    WorkspaceLoader,
};

use crate::{
    cli::{DumpArgs, DumpCommand},
    report::{
        DiagnosticView, DocumentDump, DumpOutput, IdentifierModeView, IncludeDirectiveView,
        ReferenceView, ResolvedIncludeView, SymbolView, WorkspaceDocumentView, WorkspaceDump,
        current_working_directory, display_path, document_display_path, document_id_display,
    },
};

/// Runs the selected `dump` subcommand.
pub fn run(arguments: &DumpArgs) -> Result<DumpOutput> {
    match &arguments.command {
        DumpCommand::Document(arguments) => dump_document(arguments.path.as_path()),
        DumpCommand::Workspace(arguments) => dump_workspace(&arguments.roots()),
    }
}

fn dump_document(path: &std::path::Path) -> Result<DumpOutput> {
    let cwd = current_working_directory()
        .context("while attempting to determine the CLI display root")?;
    let canonical_path = fs::canonicalize(path)
        .with_context(|| format!("while attempting to resolve {}", path.display()))?;
    let source = fs::read_to_string(&canonical_path)
        .with_context(|| format!("while attempting to read {}", canonical_path.display()))?;

    let mut analyzer = DocumentAnalyzer::new();
    let snapshot = analyzer.analyze(
        DocumentInput::new(canonical_path.to_string_lossy().into_owned(), source)
            .with_location(canonical_path.clone()),
    );

    let display_path = document_display_path(snapshot.location(), snapshot.id(), &cwd);
    let syntax_diagnostics = snapshot
        .syntax_diagnostics()
        .iter()
        .map(|diagnostic| DiagnosticView::from_analysis(display_path.clone(), diagnostic))
        .collect();
    let include_directives = snapshot
        .include_directives()
        .iter()
        .map(|directive| IncludeDirectiveView {
            raw_value: directive.raw_value.clone(),
            value_kind: directive_value_kind_name(&directive.value_kind),
            container: directive_container_name(&directive.container),
            span: directive.span.into(),
            value_span: directive.value_span.into(),
        })
        .collect();
    let identifier_modes = snapshot
        .identifier_modes()
        .iter()
        .map(|fact| IdentifierModeView {
            mode: identifier_mode_name(&fact.mode),
            raw_value: fact.raw_value.clone(),
            value_kind: directive_value_kind_name(&fact.value_kind),
            container: directive_container_name(&fact.container),
            span: fact.span.into(),
            value_span: fact.value_span.into(),
        })
        .collect();
    let symbols = snapshot
        .symbols()
        .iter()
        .map(|symbol| SymbolView {
            id: symbol.id.0,
            kind: symbol_kind_name(symbol.kind),
            display_name: symbol.display_name.clone(),
            binding_name: symbol.binding_name.clone(),
            span: symbol.span.into(),
            parent: symbol.parent.map(|parent| parent.0),
            syntax_node_kind: symbol.syntax_node_kind.clone(),
        })
        .collect();
    let references = snapshot
        .references()
        .iter()
        .map(|reference| ReferenceView {
            kind: reference_kind_name(reference.kind),
            raw_text: reference.raw_text.clone(),
            span: reference.span.into(),
            target_hint: target_hint_name(reference.target_hint),
            container_node_kind: reference.container_node_kind.clone(),
            containing_symbol: reference.containing_symbol.map(|symbol| symbol.0),
        })
        .collect();

    Ok(DumpOutput::Document(DocumentDump {
        path: display_path,
        workspace_entry: snapshot.is_workspace_entry(),
        syntax_diagnostics,
        include_directives,
        identifier_modes,
        symbols,
        references,
    }))
}

fn dump_workspace(roots: &[std::path::PathBuf]) -> Result<DumpOutput> {
    let cwd = current_working_directory()
        .context("while attempting to determine the CLI display root")?;
    let display_roots = roots
        .iter()
        .map(|root| root.display().to_string())
        .collect::<Vec<_>>();

    let mut loader = WorkspaceLoader::new();
    let workspace = loader.load_paths(roots).with_context(|| {
        format!(
            "while attempting to load workspace roots: {}",
            roots
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let document_paths = workspace
        .documents()
        .iter()
        .map(|document| {
            (
                document.id().as_str().to_owned(),
                document_display_path(document.snapshot().location(), document.id(), &cwd),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let documents = workspace_document_views(&workspace, &cwd);
    let includes = resolved_include_views(&workspace, &document_paths, &cwd);
    let include_diagnostics = include_diagnostic_views(&workspace, &document_paths);

    let entry_documents = workspace
        .entry_documents()
        .map(|document| document_display_path(document.snapshot().location(), document.id(), &cwd))
        .collect();

    Ok(DumpOutput::Workspace(WorkspaceDump {
        roots: display_roots,
        entry_documents,
        documents,
        includes,
        include_diagnostics,
    }))
}
fn workspace_document_views(
    workspace: &WorkspaceFacts,
    cwd: &std::path::Path,
) -> Vec<WorkspaceDocumentView> {
    workspace
        .documents()
        .iter()
        .map(|document| workspace_document_view(document, cwd))
        .collect()
}

fn workspace_document_view(
    document: &WorkspaceDocument,
    cwd: &std::path::Path,
) -> WorkspaceDocumentView {
    let path = document_display_path(document.snapshot().location(), document.id(), cwd);

    WorkspaceDocumentView {
        path: path.clone(),
        kind: workspace_document_kind_name(document.kind()),
        discovered_by_scan: document.discovered_by_scan(),
        syntax_diagnostics: document
            .snapshot()
            .syntax_diagnostics()
            .iter()
            .map(|diagnostic| DiagnosticView::from_analysis(path.clone(), diagnostic))
            .collect(),
        include_directive_count: document.snapshot().include_directives().len(),
        symbol_count: document.snapshot().symbols().len(),
        reference_count: document.snapshot().references().len(),
    }
}

fn resolved_include_views(
    workspace: &WorkspaceFacts,
    document_paths: &std::collections::BTreeMap<String, String>,
    cwd: &std::path::Path,
) -> Vec<ResolvedIncludeView> {
    workspace
        .includes()
        .iter()
        .map(|include| resolved_include_view(include, document_paths, cwd))
        .collect()
}

fn resolved_include_view(
    include: &ResolvedInclude,
    document_paths: &std::collections::BTreeMap<String, String>,
    cwd: &std::path::Path,
) -> ResolvedIncludeView {
    ResolvedIncludeView {
        document: document_paths
            .get(include.including_document().as_str())
            .cloned()
            .unwrap_or_else(|| document_id_display(include.including_document())),
        target_kind: workspace_include_target_kind_name(include.target()),
        target_text: include.target_text().to_owned(),
        raw_value: include.raw_value().to_owned(),
        span: include.span().into(),
        value_span: include.value_span().into(),
        target_location: workspace_include_target_location(include.target(), cwd),
        discovered_documents: include
            .discovered_documents()
            .iter()
            .map(|document_id| {
                document_paths
                    .get(document_id.as_str())
                    .cloned()
                    .unwrap_or_else(|| document_id_display(document_id))
            })
            .collect(),
    }
}

fn include_diagnostic_views(
    workspace: &WorkspaceFacts,
    document_paths: &std::collections::BTreeMap<String, String>,
) -> Vec<DiagnosticView> {
    workspace
        .include_diagnostics()
        .iter()
        .map(|diagnostic| include_diagnostic_view(diagnostic, document_paths))
        .collect()
}

fn include_diagnostic_view(
    diagnostic: &RuledDiagnostic,
    document_paths: &std::collections::BTreeMap<String, String>,
) -> DiagnosticView {
    let document = diagnostic
        .document()
        .expect("workspace include diagnostics should carry documents");
    let path = document_paths
        .get(document.as_str())
        .cloned()
        .unwrap_or_else(|| document_id_display(document));

    DiagnosticView::from_analysis(path, diagnostic)
}

fn directive_value_kind_name(kind: &DirectiveValueKind) -> String {
    match kind {
        DirectiveValueKind::BareValue => "bare_value".to_owned(),
        DirectiveValueKind::Identifier => "identifier".to_owned(),
        DirectiveValueKind::String => "string".to_owned(),
        DirectiveValueKind::TextBlockString => "text_block_string".to_owned(),
        DirectiveValueKind::Other(other) => other.clone(),
    }
}

fn directive_container_name(container: &DirectiveContainer) -> String {
    match container {
        DirectiveContainer::SourceFile => "source_file".to_owned(),
        DirectiveContainer::Workspace => "workspace".to_owned(),
        DirectiveContainer::Model => "model".to_owned(),
        DirectiveContainer::Other(other) => other.clone(),
    }
}

fn identifier_mode_name(mode: &IdentifierMode) -> String {
    match mode {
        IdentifierMode::Flat => "flat".to_owned(),
        IdentifierMode::Hierarchical => "hierarchical".to_owned(),
        IdentifierMode::Other(other) => other.clone(),
    }
}

fn symbol_kind_name(kind: SymbolKind) -> String {
    match kind {
        SymbolKind::Person => "person".to_owned(),
        SymbolKind::SoftwareSystem => "software_system".to_owned(),
        SymbolKind::Container => "container".to_owned(),
        SymbolKind::Component => "component".to_owned(),
        SymbolKind::DeploymentEnvironment => "deployment_environment".to_owned(),
        SymbolKind::DeploymentNode => "deployment_node".to_owned(),
        SymbolKind::InfrastructureNode => "infrastructure_node".to_owned(),
        SymbolKind::ContainerInstance => "container_instance".to_owned(),
        SymbolKind::SoftwareSystemInstance => "software_system_instance".to_owned(),
        SymbolKind::Relationship => "relationship".to_owned(),
    }
}

fn reference_kind_name(kind: ReferenceKind) -> String {
    match kind {
        ReferenceKind::RelationshipSource => "relationship_source".to_owned(),
        ReferenceKind::RelationshipDestination => "relationship_destination".to_owned(),
        ReferenceKind::InstanceTarget => "instance_target".to_owned(),
        ReferenceKind::DeploymentRelationshipSource => "deployment_relationship_source".to_owned(),
        ReferenceKind::DeploymentRelationshipDestination => {
            "deployment_relationship_destination".to_owned()
        }
        ReferenceKind::ViewScope => "view_scope".to_owned(),
        ReferenceKind::ViewInclude => "view_include".to_owned(),
        ReferenceKind::ViewExclude => "view_exclude".to_owned(),
        ReferenceKind::ViewAnimation => "view_animation".to_owned(),
    }
}

fn target_hint_name(hint: ReferenceTargetHint) -> String {
    match hint {
        ReferenceTargetHint::Element => "element".to_owned(),
        ReferenceTargetHint::Deployment => "deployment".to_owned(),
        ReferenceTargetHint::Relationship => "relationship".to_owned(),
        ReferenceTargetHint::ElementOrRelationship => "element_or_relationship".to_owned(),
    }
}

fn workspace_document_kind_name(kind: WorkspaceDocumentKind) -> String {
    match kind {
        WorkspaceDocumentKind::Entry => "entry".to_owned(),
        WorkspaceDocumentKind::Fragment => "fragment".to_owned(),
    }
}

fn workspace_include_target_kind_name(target: &WorkspaceIncludeTarget) -> String {
    match target {
        WorkspaceIncludeTarget::LocalFile { .. } => "local_file".to_owned(),
        WorkspaceIncludeTarget::LocalDirectory { .. } => "local_directory".to_owned(),
        WorkspaceIncludeTarget::RemoteUrl { .. } => "remote_url".to_owned(),
        WorkspaceIncludeTarget::MissingLocalPath { .. } => "missing_local_path".to_owned(),
        WorkspaceIncludeTarget::UnsupportedLocalPath { .. } => "unsupported_local_path".to_owned(),
    }
}

fn workspace_include_target_location(
    target: &WorkspaceIncludeTarget,
    cwd: &std::path::Path,
) -> String {
    match target {
        WorkspaceIncludeTarget::LocalFile { path }
        | WorkspaceIncludeTarget::LocalDirectory { path }
        | WorkspaceIncludeTarget::MissingLocalPath { path }
        | WorkspaceIncludeTarget::UnsupportedLocalPath { path } => display_path(path, cwd),
        WorkspaceIncludeTarget::RemoteUrl { url } => url.clone(),
    }
}
