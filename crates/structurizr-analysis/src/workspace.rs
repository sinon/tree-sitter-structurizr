//! Workspace discovery, include-following, and file-level include diagnostics.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
};

use ignore::WalkBuilder;

use crate::{
    ConstantDefinition, DocumentAnalyzer, DocumentId, DocumentInput, IncludeDiagnostic,
    IncludeDirective, TextSpan, includes::normalized_directive_value,
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
    snapshot: crate::DocumentSnapshot,
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
    pub const fn snapshot(&self) -> &crate::DocumentSnapshot {
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

    /// Returns the normalized target text after string substitution.
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
    include_diagnostics: Vec<IncludeDiagnostic>,
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

    /// Returns include-resolution diagnostics in deterministic order.
    #[must_use]
    pub fn include_diagnostics(&self) -> &[IncludeDiagnostic] {
        &self.include_diagnostics
    }

    /// Returns include-resolution diagnostics for one document.
    pub fn include_diagnostics_for(
        &self,
        id: &DocumentId,
    ) -> impl Iterator<Item = &IncludeDiagnostic> + '_ {
        let id = id.clone();
        self.include_diagnostics
            .iter()
            .filter(move |diagnostic| diagnostic.document == id)
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
    document_overrides: BTreeMap<PathBuf, String>,
}

impl WorkspaceLoader {
    /// Creates a loader with a reusable parser-backed document analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an in-memory source override for one canonical file path.
    ///
    /// This is useful for editor integrations that need workspace loading to
    /// observe unsaved buffer contents instead of only on-disk text.
    pub fn set_document_override(&mut self, path: PathBuf, source: String) {
        self.document_overrides.insert(path, source);
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

        let mut loaded_documents = BTreeMap::<PathBuf, WorkspaceDocument>::new();
        for root in &normalized_roots {
            for path in scan_workspace_root(root)? {
                self.load_document(path, true, &mut loaded_documents)?;
            }
        }

        let mut processed_contexts =
            BTreeMap::<DocumentContextKey, ProcessedDocumentContext>::new();
        let mut active_stack = Vec::new();

        for context in start_contexts(&normalized_roots, &loaded_documents) {
            let _ = self.process_document_context(
                context,
                &mut loaded_documents,
                &mut processed_contexts,
                &mut active_stack,
            )?;
        }

        let mut resolved_includes = processed_contexts
            .into_values()
            .flat_map(|context| context.direct_includes)
            .collect::<Vec<_>>();

        resolved_includes.sort_by(|left, right| {
            left.including_document()
                .as_str()
                .cmp(right.including_document().as_str())
                .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
                .then_with(|| left.target_text().cmp(right.target_text()))
        });
        let include_diagnostics = include_diagnostics(&resolved_includes);

        Ok(WorkspaceFacts {
            documents: loaded_documents.into_values().collect(),
            resolved_includes,
            include_diagnostics,
        })
    }

    fn load_document(
        &mut self,
        path: PathBuf,
        discovered_by_scan: bool,
        loaded_documents: &mut BTreeMap<PathBuf, WorkspaceDocument>,
    ) -> io::Result<()> {
        if let Some(document) = loaded_documents.get_mut(&path) {
            if discovered_by_scan {
                document.mark_discovered_by_scan();
            }
            return Ok(());
        }

        // Prefer unsaved editor text when one has been registered for this
        // document, and only hit the filesystem when discovery has no override.
        let source = if let Some(source) = self.document_overrides.get(&path).cloned() {
            source
        } else {
            fs::read_to_string(&path)?
        };
        let snapshot = self.analyzer.analyze(
            DocumentInput::new(document_id_from_path(&path), source).with_location(path.clone()),
        );
        let kind = if snapshot.is_workspace_entry() {
            WorkspaceDocumentKind::Entry
        } else {
            WorkspaceDocumentKind::Fragment
        };

        loaded_documents.insert(
            path,
            WorkspaceDocument {
                snapshot,
                kind,
                discovered_by_scan,
            },
        );
        Ok(())
    }

    fn process_document_context(
        &mut self,
        context: DocumentContext,
        loaded_documents: &mut BTreeMap<PathBuf, WorkspaceDocument>,
        processed_contexts: &mut BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
        active_stack: &mut Vec<PathBuf>,
    ) -> io::Result<ConstantEnvironment> {
        if let Some(processed_context) = processed_contexts.get(&context.key) {
            return Ok(processed_context.exported_constants.clone());
        }

        self.load_document(context.path.clone(), false, loaded_documents)?;

        let (document_id, constant_definitions, include_directives) = {
            let document = loaded_documents
                .get(&context.path)
                .expect("BUG: document context should be loaded before processing");

            (
                document.id().clone(),
                document.snapshot().constant_definitions().to_vec(),
                document.snapshot().include_directives().to_vec(),
            )
        };

        active_stack.push(context.path.clone());
        let processed = (|| -> io::Result<ProcessedDocumentContext> {
            let mut current_constants = context.inherited_constants.clone();
            let mut direct_includes = Vec::new();

            // Process constants and includes in source order so inherited values,
            // local definitions, and included fragments all obey the DSL's
            // imperative execution model.
            for event in document_directive_events(&constant_definitions, &include_directives) {
                match event {
                    DocumentDirectiveEvent::Constant(constant) => {
                        apply_constant_definition(constant, &mut current_constants);
                    }
                    DocumentDirectiveEvent::Include(directive) => {
                        let resolved_include = resolve_include(
                            &document_id,
                            &context.path,
                            directive,
                            &current_constants,
                        )?;

                        for included_path in &resolved_include.discovered_paths {
                            self.load_document(included_path.clone(), false, loaded_documents)?;
                        }

                        for included_path in &resolved_include.discovered_paths {
                            if active_stack.contains(included_path) {
                                continue;
                            }

                            let child_context = DocumentContext::new(
                                included_path.clone(),
                                current_constants.clone(),
                            );
                            current_constants = self.process_document_context(
                                child_context,
                                loaded_documents,
                                processed_contexts,
                                active_stack,
                            )?;
                        }

                        direct_includes.push(resolved_include.include);
                    }
                }
            }

            Ok(ProcessedDocumentContext {
                exported_constants: current_constants,
                direct_includes,
            })
        })();
        let popped_path = active_stack.pop();
        debug_assert_eq!(popped_path.as_deref(), Some(context.path.as_path()));

        let processed = processed?;
        let exported_constants = processed.exported_constants.clone();
        processed_contexts.insert(context.key, processed);
        Ok(exported_constants)
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ConstantEnvironment {
    bindings: BTreeMap<String, String>,
}

impl ConstantEnvironment {
    fn insert(&mut self, name: String, value: String) {
        self.bindings.insert(name, value);
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.bindings.get(name).map(String::as_str)
    }

    fn context_key_entries(&self) -> Vec<(String, String)> {
        self.bindings
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }
}

#[derive(Debug, Clone)]
struct DocumentContext {
    key: DocumentContextKey,
    path: PathBuf,
    inherited_constants: ConstantEnvironment,
}

impl DocumentContext {
    fn new(path: PathBuf, inherited_constants: ConstantEnvironment) -> Self {
        let key = DocumentContextKey {
            path: path.clone(),
            inherited_constants: inherited_constants.context_key_entries(),
        };

        Self {
            key,
            path,
            inherited_constants,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DocumentContextKey {
    path: PathBuf,
    inherited_constants: Vec<(String, String)>,
}

#[derive(Debug)]
struct ProcessedDocumentContext {
    exported_constants: ConstantEnvironment,
    direct_includes: Vec<ResolvedInclude>,
}

enum DocumentDirectiveEvent<'a> {
    Constant(&'a ConstantDefinition),
    Include(&'a IncludeDirective),
}

impl DocumentDirectiveEvent<'_> {
    const fn sort_rank(&self) -> usize {
        match self {
            Self::Constant(_) => 0,
            Self::Include(_) => 1,
        }
    }

    const fn start_byte(&self) -> usize {
        match self {
            Self::Constant(constant) => constant.span.start_byte,
            Self::Include(include) => include.span.start_byte,
        }
    }
}

fn start_contexts(
    normalized_roots: &[PathBuf],
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
) -> Vec<DocumentContext> {
    let mut contexts = Vec::new();
    let mut seen = BTreeSet::new();

    for root in normalized_roots {
        for path in start_paths_for_root(root, loaded_documents) {
            let context = DocumentContext::new(path, ConstantEnvironment::default());
            if seen.insert(context.key.clone()) {
                contexts.push(context);
            }
        }
    }

    contexts.sort_by(|left, right| left.key.cmp(&right.key));
    contexts
}

fn start_paths_for_root(
    root: &Path,
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }

    let mut paths = documents_under_root(root, loaded_documents, |document| {
        document.kind() == WorkspaceDocumentKind::Entry
    });
    if paths.is_empty() {
        paths = documents_under_root(root, loaded_documents, |_| true);
    }
    paths
}

fn documents_under_root<F>(
    root: &Path,
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
    predicate: F,
) -> Vec<PathBuf>
where
    F: Fn(&WorkspaceDocument) -> bool,
{
    let mut paths = loaded_documents
        .iter()
        .filter_map(|(path, document)| {
            (path.starts_with(root) && predicate(document)).then_some(path.clone())
        })
        .collect::<Vec<_>>();

    paths.sort();
    paths.dedup();
    paths
}

fn document_directive_events<'a>(
    constant_definitions: &'a [ConstantDefinition],
    include_directives: &'a [IncludeDirective],
) -> Vec<DocumentDirectiveEvent<'a>> {
    let mut events = constant_definitions
        .iter()
        .map(DocumentDirectiveEvent::Constant)
        .chain(
            include_directives
                .iter()
                .map(DocumentDirectiveEvent::Include),
        )
        .collect::<Vec<_>>();

    events.sort_by(|left, right| {
        left.start_byte()
            .cmp(&right.start_byte())
            .then_with(|| left.sort_rank().cmp(&right.sort_rank()))
    });
    events
}

fn apply_constant_definition(
    constant: &ConstantDefinition,
    current_constants: &mut ConstantEnvironment,
) {
    let expanded_value = expand_string_substitutions(&constant.value, current_constants);
    current_constants.insert(constant.name.clone(), expanded_value);
}

fn normalize_existing_path(path: &Path) -> io::Result<PathBuf> {
    fs::canonicalize(path)
}

fn scan_workspace_root(root: &Path) -> io::Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut builder = WalkBuilder::new(root);
    builder.sort_by_file_path(std::cmp::Ord::cmp);

    let mut paths = Vec::new();

    for entry in builder.build() {
        let entry = entry.map_err(io::Error::other)?;
        let entry_path = entry.path();
        let is_file = entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file());

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

fn resolve_include(
    including_document: &DocumentId,
    including_document_path: &Path,
    directive: &IncludeDirective,
    constants: &ConstantEnvironment,
) -> io::Result<ResolvedIncludeWork> {
    let target_text = expand_string_substitutions(&normalized_include_value(directive), constants);
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
            WorkspaceIncludeTarget::UnsupportedLocalPath {
                path: joined_target,
            },
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
            WorkspaceIncludeTarget::UnsupportedLocalPath {
                path: joined_target,
            },
            Vec::new(),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(base_include(
            WorkspaceIncludeTarget::MissingLocalPath {
                path: joined_target,
            },
            Vec::new(),
        )),
        Err(error) => Err(error),
    }
}

fn normalized_include_value(directive: &IncludeDirective) -> String {
    normalized_directive_value(&directive.raw_value, &directive.value_kind)
}

fn expand_string_substitutions(value: &str, constants: &ConstantEnvironment) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut remaining = value;

    while let Some(marker_start) = remaining.find("${") {
        expanded.push_str(&remaining[..marker_start]);

        let placeholder = &remaining[marker_start..];
        let Some(placeholder_end) = placeholder.find('}') else {
            expanded.push_str(placeholder);
            return expanded;
        };

        let name = &placeholder[2..placeholder_end];
        if is_supported_substitution_name(name) {
            if let Some(replacement) = constants.get(name) {
                expanded.push_str(replacement);
            } else {
                expanded.push_str(&placeholder[..=placeholder_end]);
            }
        } else {
            expanded.push_str(&placeholder[..=placeholder_end]);
        }

        remaining = &placeholder[placeholder_end + 1..];
    }

    expanded.push_str(remaining);
    expanded
}

fn is_supported_substitution_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
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
        let is_file = entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file());

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

fn include_diagnostics(resolved_includes: &[ResolvedInclude]) -> Vec<IncludeDiagnostic> {
    let cycle_include_indices = cycle_include_indices(resolved_includes);
    let mut diagnostics = Vec::new();

    for (index, include) in resolved_includes.iter().enumerate() {
        match include.target() {
            WorkspaceIncludeTarget::MissingLocalPath { .. } => {
                diagnostics.push(IncludeDiagnostic::missing_local_target(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::UnsupportedLocalPath { .. } => {
                diagnostics.push(IncludeDiagnostic::escapes_allowed_subtree(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::RemoteUrl { .. } => {
                diagnostics.push(IncludeDiagnostic::unsupported_remote_target(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::LocalFile { .. }
            | WorkspaceIncludeTarget::LocalDirectory { .. } => {}
        }

        if cycle_include_indices.contains(&index) {
            diagnostics.push(IncludeDiagnostic::include_cycle(
                include.including_document(),
                include.target_text(),
                include.span(),
                include.value_span(),
            ));
        }
    }

    diagnostics.sort_by(|left, right| {
        left.document
            .cmp(&right.document)
            .then_with(|| left.span.start_byte.cmp(&right.span.start_byte))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.target_text.cmp(&right.target_text))
    });
    diagnostics
}

fn cycle_include_indices(resolved_includes: &[ResolvedInclude]) -> BTreeSet<usize> {
    let mut adjacency = BTreeMap::<DocumentId, Vec<(usize, DocumentId)>>::new();

    for (index, include) in resolved_includes.iter().enumerate() {
        for discovered_document in include.discovered_documents() {
            adjacency
                .entry(include.including_document().clone())
                .or_default()
                .push((index, discovered_document.clone()));
        }
    }

    let mut cycle_indices = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut stack_documents = Vec::new();
    let mut stack_edges = Vec::new();

    for document in adjacency.keys() {
        collect_cycle_include_indices(
            document,
            &adjacency,
            &mut visited,
            &mut stack_documents,
            &mut stack_edges,
            &mut cycle_indices,
        );
    }

    cycle_indices
}

fn collect_cycle_include_indices(
    document: &DocumentId,
    adjacency: &BTreeMap<DocumentId, Vec<(usize, DocumentId)>>,
    visited: &mut BTreeSet<DocumentId>,
    stack_documents: &mut Vec<DocumentId>,
    stack_edges: &mut Vec<usize>,
    cycle_indices: &mut BTreeSet<usize>,
) {
    if visited.contains(document) {
        return;
    }

    stack_documents.push(document.clone());

    if let Some(edges) = adjacency.get(document) {
        for (edge_index, child) in edges {
            if let Some(stack_index) = stack_documents
                .iter()
                .position(|candidate| candidate == child)
            {
                for cycle_edge in stack_edges.iter().skip(stack_index) {
                    cycle_indices.insert(*cycle_edge);
                }
                cycle_indices.insert(*edge_index);
                continue;
            }

            if visited.contains(child) {
                continue;
            }

            stack_edges.push(*edge_index);
            collect_cycle_include_indices(
                child,
                adjacency,
                visited,
                stack_documents,
                stack_edges,
                cycle_indices,
            );
            let _ = stack_edges.pop();
        }
    }

    let _ = stack_documents.pop();
    visited.insert(document.clone());
}
