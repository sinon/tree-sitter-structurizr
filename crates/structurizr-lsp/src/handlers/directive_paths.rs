//! Shared resolution for directive arguments that point at local paths.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use structurizr_analysis::{
    DocumentId, DocumentSnapshot, TextSpan, WorkspaceFacts, WorkspaceIncludeTarget,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DirectivePathKind {
    Docs,
    Adrs,
    IncludeFile,
    IncludeDirectory,
}

impl DirectivePathKind {
    pub(super) const fn tooltip(self) -> &'static str {
        match self {
            Self::Docs => "Open documentation folder",
            Self::Adrs => "Open ADRs folder",
            Self::IncludeFile => "Open included file",
            Self::IncludeDirectory => "Open included directory",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ResolvedDirectivePath {
    span: TextSpan,
    path: PathBuf,
    kind: DirectivePathKind,
}

impl ResolvedDirectivePath {
    const fn new(span: TextSpan, path: PathBuf, kind: DirectivePathKind) -> Self {
        Self { span, path, kind }
    }

    pub(super) const fn span(&self) -> TextSpan {
        self.span
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) const fn kind(&self) -> DirectivePathKind {
        self.kind
    }
}

#[must_use]
pub(super) fn resolved_directive_paths(
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
) -> Vec<ResolvedDirectivePath> {
    let mut paths = docs_and_adrs_paths(snapshot);
    paths.extend(include_paths(snapshot, workspace_facts));
    paths.sort();
    paths.dedup();
    paths
}

#[must_use]
pub(super) fn resolved_directive_paths_at_offset(
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
    offset: usize,
) -> Vec<ResolvedDirectivePath> {
    resolved_directive_paths(snapshot, workspace_facts)
        .into_iter()
        .filter(|path| span_contains(path.span.start_byte, path.span.end_byte, offset))
        .collect()
}

fn docs_and_adrs_paths(snapshot: &DocumentSnapshot) -> Vec<ResolvedDirectivePath> {
    let Some(base_dir) = snapshot
        .location()
        .and_then(|location| location.path().parent())
    else {
        return Vec::new();
    };
    let source = snapshot.source().as_bytes();
    let mut stack = vec![snapshot.tree().root_node()];
    let mut paths = Vec::new();

    while let Some(node) = stack.pop() {
        let Some(kind) = (match node.kind() {
            "docs_directive" => Some(DirectivePathKind::Docs),
            "adrs_directive" => Some(DirectivePathKind::Adrs),
            _ => None,
        }) else {
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
            continue;
        };

        let Some(path_node) = node.child_by_field_name("path") else {
            continue;
        };
        let Ok(raw_value) = path_node.utf8_text(source) else {
            continue;
        };
        let normalized = normalized_directive_path(raw_value, path_node.kind());
        let Some(target_path) = resolve_existing_path(base_dir, &normalized) else {
            continue;
        };

        paths.push(ResolvedDirectivePath::new(
            TextSpan::from_node(path_node),
            target_path,
            kind,
        ));
    }

    paths
}

fn include_paths(
    snapshot: &DocumentSnapshot,
    workspace_facts: Option<&WorkspaceFacts>,
) -> Vec<ResolvedDirectivePath> {
    let Some(workspace_facts) = workspace_facts else {
        return Vec::new();
    };
    let Some(document_id) = workspace_document_id(snapshot) else {
        return Vec::new();
    };
    let mut targets = BTreeSet::<ResolvedDirectivePath>::new();

    for include in workspace_facts
        .includes()
        .iter()
        .filter(|include| include.including_document() == &document_id)
    {
        let (path, kind) = match include.target() {
            WorkspaceIncludeTarget::LocalFile { path } => {
                (path.clone(), DirectivePathKind::IncludeFile)
            }
            WorkspaceIncludeTarget::LocalDirectory { path } => {
                (path.clone(), DirectivePathKind::IncludeDirectory)
            }
            WorkspaceIncludeTarget::RemoteUrl { .. }
            | WorkspaceIncludeTarget::MissingLocalPath { .. }
            | WorkspaceIncludeTarget::UnsupportedLocalPath { .. } => continue,
        };

        targets.insert(ResolvedDirectivePath::new(include.value_span(), path, kind));
    }

    targets.into_iter().collect()
}

fn workspace_document_id(snapshot: &DocumentSnapshot) -> Option<DocumentId> {
    let path = snapshot.location()?.path();
    let canonical_path = fs::canonicalize(path).ok()?;
    Some(DocumentId::new(
        canonical_path.to_string_lossy().into_owned(),
    ))
}

fn resolve_existing_path(base_dir: &Path, target_text: &str) -> Option<PathBuf> {
    if target_text.is_empty()
        || target_text.starts_with("http://")
        || target_text.starts_with("https://")
    {
        return None;
    }

    fs::canonicalize(base_dir.join(target_text)).ok()
}

fn normalized_directive_path(raw_value: &str, node_kind: &str) -> String {
    match node_kind {
        "string" => strip_wrapping(raw_value, "\"", "\"").to_owned(),
        "text_block_string" => strip_wrapping(raw_value, "\"\"\"", "\"\"\"").to_owned(),
        _ => raw_value.to_owned(),
    }
}

fn strip_wrapping<'a>(value: &'a str, prefix: &str, suffix: &str) -> &'a str {
    value
        .strip_prefix(prefix)
        .and_then(|stripped| stripped.strip_suffix(suffix))
        .unwrap_or(value)
}

const fn span_contains(start_byte: usize, end_byte: usize, offset: usize) -> bool {
    if start_byte == end_byte {
        offset == start_byte
    } else {
        start_byte <= offset && offset < end_byte
    }
}
