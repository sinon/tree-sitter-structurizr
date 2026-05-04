// Include and workspace-base resolution, constant propagation, filesystem
// helpers, and the imperative document-context processing layer.

#[derive(Debug)]
struct ResolvedIncludeWork {
    include: ResolvedInclude,
    discovered_paths: Vec<PathBuf>,
}

#[derive(Debug)]
struct ResolvedWorkspaceBase {
    path: PathBuf,
    target_text: String,
}

#[derive(Debug, Clone)]
struct WorkspaceBaseDirective {
    raw_value: String,
    value_kind: DirectiveValueKind,
    span: TextSpan,
    value_span: TextSpan,
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

#[derive(Debug, Clone)]
struct ProcessedDocumentContext {
    exported_constants: ConstantEnvironment,
    workspace_base_context: Option<DocumentContextKey>,
    direct_includes: Vec<ResolvedInclude>,
    included_contexts: Vec<DocumentContextKey>,
}

fn collect_document_directive_events(
    snapshot: &crate::DocumentSnapshot,
) -> Vec<WorkspaceDirectiveEvent> {
    let constant_definitions = snapshot.constant_definitions();
    let include_directives = snapshot.include_directives();

    // Both extractors walk the tree in source order already, so the workspace
    // cache can merge the two ordered streams once and replay the result across
    // repeated context processing without paying another sort.
    let mut events = Vec::with_capacity(constant_definitions.len() + include_directives.len());
    let mut constant_index = 0;
    let mut include_index = 0;

    while constant_index < constant_definitions.len() && include_index < include_directives.len() {
        let constant = &constant_definitions[constant_index];
        let include = &include_directives[include_index];

        if constant.span.start_byte <= include.span.start_byte {
            events.push(WorkspaceDirectiveEvent::ConstantDefinition(constant_index));
            constant_index += 1;
        } else {
            events.push(WorkspaceDirectiveEvent::IncludeDirective(include_index));
            include_index += 1;
        }
    }

    while constant_index < constant_definitions.len() {
        events.push(WorkspaceDirectiveEvent::ConstantDefinition(constant_index));
        constant_index += 1;
    }

    while include_index < include_directives.len() {
        events.push(WorkspaceDirectiveEvent::IncludeDirective(include_index));
        include_index += 1;
    }

    events
}

fn processed_context_dependency_keys(
    processed: &ProcessedDocumentContext,
) -> impl Iterator<Item = &DocumentContextKey> {
    processed
        .workspace_base_context
        .iter()
        .chain(processed.included_contexts.iter())
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

#[allow(clippy::too_many_lines)]
fn resolve_include(
    including_document: &DocumentId,
    including_document_path: &Path,
    directive: &IncludeDirective,
    constants: &ConstantEnvironment,
) -> WorkspaceLoadResult<ResolvedIncludeWork> {
    let target_text = expand_string_substitutions(&normalized_include_value(directive), constants);
    let include_error = |path: Option<PathBuf>, error: io::Error| {
        WorkspaceLoadError::single(WorkspaceLoadFailure::include_load(
            including_document,
            directive.span,
            directive.value_span,
            &target_text,
            path,
            &error,
        ))
    };
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
        return Err(include_error(
            Some(including_document_path.to_path_buf()),
            io::Error::other(format!(
                "document path has no parent directory: {}",
                including_document_path.display()
            )),
        ));
    };
    let canonical_parent_directory = normalize_existing_path(parent_directory)
        .map_err(|error| include_error(Some(parent_directory.to_path_buf()), error))?;
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
            let canonical_file = normalize_existing_path(&joined_target)
                .map_err(|error| include_error(Some(joined_target.clone()), error))?;

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
            let canonical_directory = normalize_existing_path(&joined_target)
                .map_err(|error| include_error(Some(joined_target.clone()), error))?;

            if !canonical_directory.starts_with(&canonical_parent_directory) {
                return Ok(base_include(
                    WorkspaceIncludeTarget::UnsupportedLocalPath {
                        path: canonical_directory,
                    },
                    Vec::new(),
                ));
            }

            let discovered_paths =
                collect_directory_include_paths(&canonical_directory, &canonical_parent_directory)
                    .map_err(|error| include_error(Some(canonical_directory.clone()), error))?;

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
        Err(error) => Err(include_error(Some(joined_target), error)),
    }
}

fn normalized_include_value(directive: &IncludeDirective) -> String {
    normalized_directive_value(&directive.raw_value, &directive.value_kind)
}

fn workspace_base_directive(snapshot: &crate::DocumentSnapshot) -> Option<WorkspaceBaseDirective> {
    let root = snapshot.tree().root_node();
    let mut cursor = root.walk();
    let workspace = root
        .named_children(&mut cursor)
        .find(|child| child.kind() == "workspace")?;
    let base = workspace.child_by_field_name("base")?;

    Some(WorkspaceBaseDirective {
        raw_value: snapshot.source()[base.byte_range()].to_owned(),
        value_kind: DirectiveValueKind::from_node_kind(base.kind()),
        span: TextSpan::from_node(base),
        value_span: TextSpan::from_node(base),
    })
}

fn resolve_workspace_base(
    document: &DocumentId,
    workspace_path: &Path,
    workspace_base: &WorkspaceBaseDirective,
    constants: &ConstantEnvironment,
) -> WorkspaceLoadResult<ResolvedWorkspaceBase> {
    let base_text = expand_string_substitutions(
        &normalized_directive_value(&workspace_base.raw_value, &workspace_base.value_kind),
        constants,
    );
    let base_error = |path: Option<PathBuf>, message: String| {
        WorkspaceLoadError::single(WorkspaceLoadFailure::workspace_base(
            document,
            workspace_base,
            &base_text,
            path,
            message,
        ))
    };
    if is_remote_include(&base_text) {
        return Err(base_error(
            None,
            format!("remote workspace bases are not supported: {base_text}"),
        ));
    }

    let parent = workspace_path.parent().ok_or_else(|| {
        base_error(
            Some(workspace_path.to_path_buf()),
            format!(
                "workspace entry has no parent directory for base resolution: {}",
                workspace_path.display()
            ),
        )
    })?;
    let canonical_parent_directory = normalize_existing_path(parent).map_err(|error| {
        base_error(
            Some(parent.to_path_buf()),
            format!("failed to load workspace base {base_text}: {error}"),
        )
    })?;
    let relative_target = PathBuf::from(&base_text);
    let base_path = parent.join(&relative_target);
    if !is_supported_local_include_path(&relative_target) {
        return Err(base_error(
            Some(base_path),
            format!("workspace base path escapes the allowed subtree: {base_text}"),
        ));
    }
    let metadata = fs::metadata(&base_path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            base_error(
                Some(base_path.clone()),
                format!("workspace base does not exist: {base_text}"),
            )
        } else {
            base_error(
                Some(base_path.clone()),
                format!("failed to load workspace base {base_text}: {error}"),
            )
        }
    })?;

    if !metadata.is_file() {
        return Err(base_error(
            Some(base_path),
            format!("workspace base must resolve to a file: {base_text}"),
        ));
    }

    let canonical_base = normalize_existing_path(&base_path).map_err(|error| {
        base_error(
            Some(base_path.clone()),
            format!("failed to load workspace base {base_text}: {error}"),
        )
    })?;
    if !canonical_base.starts_with(&canonical_parent_directory) {
        return Err(base_error(
            Some(canonical_base),
            format!("workspace base path escapes the allowed subtree: {base_text}"),
        ));
    }

    Ok(ResolvedWorkspaceBase {
        path: canonical_base,
        target_text: base_text,
    })
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

fn resolve_local_resource_path(
    document_location: Option<&DocumentLocation>,
    value: &ValueFact,
) -> Option<PathBuf> {
    if value.value_kind == DirectiveValueKind::TextBlockString
        || value.normalized_text.contains("${")
        || is_remote_resource_value(&value.normalized_text)
    {
        return None;
    }

    let relative_path = PathBuf::from(&value.normalized_text);
    if !is_supported_local_include_path(&relative_path) {
        return None;
    }

    // TODO: Expand `${...}` substitutions here once workspace semantic packets
    // carry the effective constant environment for each processed document.
    let parent_directory = document_location?.path().parent()?;
    Some(parent_directory.join(relative_path))
}

fn is_remote_resource_value(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://") || value.starts_with("data:")
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

fn include_diagnostics(resolved_includes: &[ResolvedInclude]) -> Vec<RuledDiagnostic> {
    let cycle_include_indices = cycle_include_indices(resolved_includes);
    let mut diagnostics = Vec::new();

    for (index, include) in resolved_includes.iter().enumerate() {
        match include.target() {
            WorkspaceIncludeTarget::MissingLocalPath { .. } => {
                diagnostics.push(RuledDiagnostic::missing_local_target(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::UnsupportedLocalPath { .. } => {
                diagnostics.push(RuledDiagnostic::escapes_allowed_subtree(
                    include.including_document(),
                    include.target_text(),
                    include.span(),
                    include.value_span(),
                ));
            }
            WorkspaceIncludeTarget::RemoteUrl { .. } => {
                diagnostics.push(RuledDiagnostic::unsupported_remote_target(
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
            diagnostics.push(RuledDiagnostic::include_cycle(
                include.including_document(),
                include.target_text(),
                include.span(),
                include.value_span(),
            ));
        }
    }

    diagnostics.sort_by(|left, right| {
        left.document()
            .cmp(&right.document())
            .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
            .then_with(|| left.rule.cmp(&right.rule))
            .then_with(|| left.target_text().cmp(&right.target_text()))
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

#[cfg(test)]
mod workspace_includes_tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn cached_parent_context_refreshes_when_included_child_changes() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                !include "model.dsl"
            }
        "#});
        fixture.write_model(indoc! {r#"
            model {
                user = person "User"
            }
        "#});

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.root()])
            .expect("first load should succeed");
        let first_index = first
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            first_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["user"]
        );

        fixture.write_model(indoc! {r#"
            model {
                admin = person "Admin"
            }
        "#});

        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");
        let second_index = second
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            second_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
    }

    #[test]
    fn cached_parent_context_refreshes_when_grandchild_changes() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                !include "model.dsl"
            }
        "#});
        fixture.write_model(indoc! {r#"
            model {
                !include "people.dsl"
            }
        "#});
        fixture.write_file(
            "people.dsl",
            indoc! {r#"
                model {
                    user = person "User"
                }
            "#},
        );

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.root()])
            .expect("first load should succeed");
        let first_index = first
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            first_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["user"]
        );

        fixture.write_file(
            "people.dsl",
            indoc! {r#"
                model {
                    admin = person "Admin"
                }
            "#},
        );

        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");
        let second_index = second
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        assert_eq!(
            second_index
                .unique_element_bindings()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
    }
}
