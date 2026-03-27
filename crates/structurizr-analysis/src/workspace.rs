//! Workspace discovery and explicit include-following for multi-file analysis.

use std::{
    collections::{BTreeMap, VecDeque},
    fs, io,
    path::{Component, Path, PathBuf},
};

use ignore::WalkBuilder;

use crate::{
    DirectiveValueKind, DocumentAnalyzer, DocumentId, DocumentInput, DocumentSnapshot,
    IncludeDirective, TextSpan,
};

/// Classifies whether a discovered document can act as a workspace entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceDocumentKind {
    /// A document with a top-level `workspace { ... }` block.
    Entry,
    /// A parseable fragment that does not declare a top-level workspace.
    Fragment,
}

/// One discovered document plus the metadata gathered during workspace loading.
#[derive(Debug)]
pub struct WorkspaceDocument {
    snapshot: DocumentSnapshot,
    kind: WorkspaceDocumentKind,
    discovered_by_scan: bool,
}

impl WorkspaceDocument {
    /// Returns the stable document identifier for the discovered document.
    #[must_use]
    pub const fn id(&self) -> &DocumentId {
        self.snapshot.id()
    }

    /// Returns the analyzed snapshot for the discovered document.
    #[must_use]
    pub const fn snapshot(&self) -> &DocumentSnapshot {
        &self.snapshot
    }

    /// Returns the document's role in the discovered workspace set.
    #[must_use]
    pub const fn kind(&self) -> WorkspaceDocumentKind {
        self.kind
    }

    /// Returns whether broad `.dsl` workspace scanning found this document.
    ///
    /// Documents discovered only via explicit `!include` traversal report
    /// `false` here.
    #[must_use]
    pub const fn discovered_by_scan(&self) -> bool {
        self.discovered_by_scan
    }

    const fn mark_discovered_by_scan(&mut self) {
        self.discovered_by_scan = true;
    }
}

/// The normalized target shape observed for one explicit `!include`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceIncludeTarget {
    /// A local file include that resolved to one concrete path.
    LocalFile {
        /// Canonical filesystem path to the followed local file.
        path: PathBuf,
    },
    /// A local directory include that expanded to zero or more concrete files.
    LocalDirectory {
        /// Canonical filesystem path to the followed local directory.
        path: PathBuf,
    },
    /// A remote include target that is recorded but not fetched.
    RemoteUrl {
        /// Remote HTTPS URL recorded during discovery.
        url: String,
    },
    /// A relative local target that did not exist on disk.
    MissingLocalPath {
        /// Filesystem path that did not exist when discovery attempted to follow it.
        path: PathBuf,
    },
    /// A local target that was rejected before loading, such as an absolute path
    /// or one that escapes the including document's directory tree.
    UnsupportedLocalPath {
        /// Filesystem path that discovery refused to follow.
        path: PathBuf,
    },
}

/// One include directive plus the discovery-layer result of following it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInclude {
    including_document: DocumentId,
    raw_value: String,
    target_text: String,
    span: TextSpan,
    value_span: TextSpan,
    target: WorkspaceIncludeTarget,
    discovered_documents: Vec<DocumentId>,
}

impl ResolvedInclude {
    /// Returns the document that declared this include directive.
    #[must_use]
    pub const fn including_document(&self) -> &DocumentId {
        &self.including_document
    }

    /// Returns the exact directive value text as it appeared in the document.
    #[must_use]
    pub fn raw_value(&self) -> &str {
        &self.raw_value
    }

    /// Returns the normalized target text with surrounding quotes stripped.
    #[must_use]
    pub fn target_text(&self) -> &str {
        &self.target_text
    }

    /// Returns the span of the full include directive.
    #[must_use]
    pub const fn span(&self) -> TextSpan {
        self.span
    }

    /// Returns the span of the include directive's value node.
    #[must_use]
    pub const fn value_span(&self) -> TextSpan {
        self.value_span
    }

    /// Returns the normalized target classification observed during discovery.
    #[must_use]
    pub const fn target(&self) -> &WorkspaceIncludeTarget {
        &self.target
    }

    /// Returns the documents followed from this include target, if any.
    #[must_use]
    pub fn discovered_documents(&self) -> &[DocumentId] {
        &self.discovered_documents
    }
}

/// Multi-file discovery facts gathered from one or more workspace roots.
#[derive(Debug, Default)]
pub struct WorkspaceFacts {
    documents: Vec<WorkspaceDocument>,
    resolved_includes: Vec<ResolvedInclude>,
}

impl WorkspaceFacts {
    /// Returns every discovered document in deterministic path order.
    #[must_use]
    pub fn documents(&self) -> &[WorkspaceDocument] {
        &self.documents
    }

    /// Returns the discovered include-following results in deterministic order.
    #[must_use]
    pub fn includes(&self) -> &[ResolvedInclude] {
        &self.resolved_includes
    }

    /// Returns the subset of discovered documents that can act as entry roots.
    pub fn entry_documents(&self) -> impl Iterator<Item = &WorkspaceDocument> + '_ {
        self.documents
            .iter()
            .filter(|document| document.kind() == WorkspaceDocumentKind::Entry)
    }

    /// Looks up one discovered document by document identifier.
    #[must_use]
    pub fn document(&self, id: &DocumentId) -> Option<&WorkspaceDocument> {
        self.documents.iter().find(|document| document.id() == id)
    }
}

/// Loader that scans workspace roots and follows explicit include targets.
#[derive(Default)]
pub struct WorkspaceLoader {
    analyzer: DocumentAnalyzer,
}

impl WorkspaceLoader {
    /// Creates a loader with a reusable parser-backed document analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Scans one or more workspace roots for `.dsl` files and follows explicit
    /// local include targets from discovered documents.
    ///
    /// General workspace scanning respects normal ignore rules. Explicit local
    /// include targets are followed separately and therefore bypass those broad
    /// scan filters.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the loader cannot traverse a workspace root or
    /// read one of the discovered local files.
    pub fn load_paths<I, P>(&mut self, roots: I) -> io::Result<WorkspaceFacts>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut normalized_roots = roots
            .into_iter()
            .map(|root| normalize_existing_path(root.as_ref()))
            .collect::<io::Result<Vec<_>>>()?;
        normalized_roots.sort();
        normalized_roots.dedup();

        let mut queued_documents = VecDeque::new();
        let mut scheduled_documents = BTreeMap::<PathBuf, bool>::new();
        let mut loaded_documents = BTreeMap::<PathBuf, WorkspaceDocument>::new();
        let mut resolved_includes = Vec::new();

        for root in normalized_roots {
            for path in scan_workspace_root(&root)? {
                schedule_document(
                    path,
                    true,
                    &mut queued_documents,
                    &mut scheduled_documents,
                    &mut loaded_documents,
                );
            }
        }

        while let Some(path) = queued_documents.pop_front() {
            let discovered_by_scan = scheduled_documents.remove(&path).unwrap_or(false);

            if let Some(document) = loaded_documents.get_mut(&path) {
                if discovered_by_scan {
                    document.mark_discovered_by_scan();
                }
                continue;
            }

            let source = fs::read_to_string(&path)?;
            let snapshot = self.analyzer.analyze(
                DocumentInput::new(document_id_from_path(&path), source).with_location(path.clone()),
            );
            let kind = if snapshot.is_workspace_entry() {
                WorkspaceDocumentKind::Entry
            } else {
                WorkspaceDocumentKind::Fragment
            };

            for resolved_include in
                resolve_includes(snapshot.id(), &path, snapshot.include_directives())?
            {
                for included_path in &resolved_include.discovered_paths {
                    schedule_document(
                        included_path.clone(),
                        false,
                        &mut queued_documents,
                        &mut scheduled_documents,
                        &mut loaded_documents,
                    );
                }
                resolved_includes.push(resolved_include.include);
            }

            loaded_documents.insert(
                path,
                WorkspaceDocument {
                    snapshot,
                    kind,
                    discovered_by_scan,
                },
            );
        }

        resolved_includes.sort_by(|left, right| {
            left.including_document()
                .as_str()
                .cmp(right.including_document().as_str())
                .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
                .then_with(|| left.target_text().cmp(right.target_text()))
        });

        Ok(WorkspaceFacts {
            documents: loaded_documents.into_values().collect(),
            resolved_includes,
        })
    }
}

/// Convenience helper for scanning workspace roots with a fresh loader.
///
/// # Errors
///
/// Returns an I/O error when the loader cannot traverse a workspace root or
/// read one of the discovered local files.
pub fn load_workspace<I, P>(roots: I) -> io::Result<WorkspaceFacts>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    WorkspaceLoader::new().load_paths(roots)
}

#[derive(Debug)]
struct ResolvedIncludeWork {
    include: ResolvedInclude,
    discovered_paths: Vec<PathBuf>,
}

fn schedule_document(
    path: PathBuf,
    discovered_by_scan: bool,
    queued_documents: &mut VecDeque<PathBuf>,
    scheduled_documents: &mut BTreeMap<PathBuf, bool>,
    loaded_documents: &mut BTreeMap<PathBuf, WorkspaceDocument>,
) {
    if let Some(document) = loaded_documents.get_mut(&path) {
        if discovered_by_scan {
            document.mark_discovered_by_scan();
        }
        return;
    }

    if let Some(existing_discovered_by_scan) = scheduled_documents.get_mut(&path) {
        *existing_discovered_by_scan |= discovered_by_scan;
        return;
    }

    scheduled_documents.insert(path.clone(), discovered_by_scan);
    queued_documents.push_back(path);
}

fn normalize_existing_path(path: &Path) -> io::Result<PathBuf> {
    fs::canonicalize(path)
}

fn scan_workspace_root(root: &Path) -> io::Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(if has_dsl_extension(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        });
    }

    let mut builder = WalkBuilder::new(root);
    builder.sort_by_file_path(std::cmp::Ord::cmp);

    let mut paths = Vec::new();

    for entry in builder.build() {
        let entry = entry.map_err(io::Error::other)?;
        let entry_path = entry.path();
        let is_file = entry.file_type().is_some_and(|file_type| file_type.is_file());

        if is_file && has_dsl_extension(entry_path) {
            paths.push(normalize_existing_path(entry_path)?);
        }
    }

    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn has_dsl_extension(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dsl"))
}

fn document_id_from_path(path: &Path) -> DocumentId {
    DocumentId::new(path.to_string_lossy().into_owned())
}

fn resolve_includes(
    including_document: &DocumentId,
    including_document_path: &Path,
    directives: &[IncludeDirective],
) -> io::Result<Vec<ResolvedIncludeWork>> {
    directives
        .iter()
        .map(|directive| resolve_include(including_document, including_document_path, directive))
        .collect()
}

fn resolve_include(
    including_document: &DocumentId,
    including_document_path: &Path,
    directive: &IncludeDirective,
) -> io::Result<ResolvedIncludeWork> {
    let target_text = normalized_include_value(directive);
    let base_include = |target: WorkspaceIncludeTarget, discovered_paths: Vec<PathBuf>| {
        let discovered_documents = discovered_paths
            .iter()
            .map(|path| document_id_from_path(path))
            .collect();

        ResolvedIncludeWork {
            include: ResolvedInclude {
                including_document: including_document.clone(),
                raw_value: directive.raw_value.clone(),
                target_text: target_text.clone(),
                span: directive.span,
                value_span: directive.value_span,
                target,
                discovered_documents,
            },
            discovered_paths,
        }
    };

    if is_remote_include(&target_text) {
        return Ok(base_include(
            WorkspaceIncludeTarget::RemoteUrl {
                url: target_text.clone(),
            },
            Vec::new(),
        ));
    }

    let Some(parent_directory) = including_document_path.parent() else {
        return Err(io::Error::other(format!(
            "document path has no parent directory: {}",
            including_document_path.display()
        )));
    };
    let canonical_parent_directory = normalize_existing_path(parent_directory)?;
    let relative_target = PathBuf::from(&target_text);
    let joined_target = parent_directory.join(&relative_target);

    if !is_supported_local_include_path(&relative_target) {
        return Ok(base_include(
            WorkspaceIncludeTarget::UnsupportedLocalPath { path: joined_target },
            Vec::new(),
        ));
    }

    match fs::metadata(&joined_target) {
        Ok(metadata) if metadata.is_file() => {
            let canonical_file = normalize_existing_path(&joined_target)?;

            if !canonical_file.starts_with(&canonical_parent_directory) {
                return Ok(base_include(
                    WorkspaceIncludeTarget::UnsupportedLocalPath {
                        path: canonical_file,
                    },
                    Vec::new(),
                ));
            }

            Ok(base_include(
                WorkspaceIncludeTarget::LocalFile {
                    path: canonical_file.clone(),
                },
                vec![canonical_file],
            ))
        }
        Ok(metadata) if metadata.is_dir() => {
            let canonical_directory = normalize_existing_path(&joined_target)?;

            if !canonical_directory.starts_with(&canonical_parent_directory) {
                return Ok(base_include(
                    WorkspaceIncludeTarget::UnsupportedLocalPath {
                        path: canonical_directory,
                    },
                    Vec::new(),
                ));
            }

            let discovered_paths =
                collect_directory_include_paths(&canonical_directory, &canonical_parent_directory)?;

            Ok(base_include(
                WorkspaceIncludeTarget::LocalDirectory {
                    path: canonical_directory,
                },
                discovered_paths,
            ))
        }
        Ok(_) => Ok(base_include(
            WorkspaceIncludeTarget::UnsupportedLocalPath { path: joined_target },
            Vec::new(),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(base_include(
            WorkspaceIncludeTarget::MissingLocalPath { path: joined_target },
            Vec::new(),
        )),
        Err(error) => Err(error),
    }
}

fn normalized_include_value(directive: &IncludeDirective) -> String {
    match directive.value_kind {
        DirectiveValueKind::String => strip_wrapping(&directive.raw_value, "\"", "\"").to_owned(),
        DirectiveValueKind::TextBlockString => {
            strip_wrapping(&directive.raw_value, "\"\"\"", "\"\"\"").to_owned()
        }
        DirectiveValueKind::BareValue
        | DirectiveValueKind::Identifier
        | DirectiveValueKind::Other(_) => directive.raw_value.clone(),
    }
}

fn strip_wrapping<'a>(value: &'a str, prefix: &str, suffix: &str) -> &'a str {
    value
        .strip_prefix(prefix)
        .and_then(|stripped| stripped.strip_suffix(suffix))
        .unwrap_or(value)
}

fn is_remote_include(target_text: &str) -> bool {
    target_text.starts_with("https://")
}

fn is_supported_local_include_path(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn collect_directory_include_paths(
    directory: &Path,
    allowed_root: &Path,
) -> io::Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(directory);
    builder.hidden(false);
    builder.ignore(false);
    builder.git_ignore(false);
    builder.git_global(false);
    builder.git_exclude(false);
    builder.sort_by_file_path(std::cmp::Ord::cmp);

    let mut paths = Vec::new();

    for entry in builder.build() {
        let entry = entry.map_err(io::Error::other)?;
        let entry_path = entry.path();
        let is_file = entry.file_type().is_some_and(|file_type| file_type.is_file());

        if !is_file {
            continue;
        }

        let canonical_path = normalize_existing_path(entry_path)?;
        if canonical_path.starts_with(allowed_root) {
            paths.push(canonical_path);
        }
    }

    paths.sort();
    paths.dedup();
    Ok(paths)
}
