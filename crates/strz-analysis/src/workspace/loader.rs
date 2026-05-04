// The public workspace loader facade and the top-level orchestration flow that
// scans roots, processes directive order, and assembles final workspace facts.

/// Loader that scans workspace roots and follows explicit include targets.
#[derive(Default)]
pub struct WorkspaceLoader {
    session: WorkspaceAnalysisSession,
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
        self.session.set_document_override(path, source);
    }

    /// Clears all registered in-memory source overrides.
    ///
    /// This keeps a long-lived loader aligned with the current open-buffer set.
    pub fn clear_document_overrides(&mut self) {
        self.session.clear_document_overrides();
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
    /// read one of the discovered local files. Use
    /// [`Self::load_paths_with_failures`] when callers need structured fatal
    /// failure data.
    pub fn load_paths<I, P>(&mut self, roots: I) -> io::Result<WorkspaceFacts>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        self.load_paths_with_failures(roots)
            .map_err(io::Error::other)
    }

    /// Scans workspace roots while preserving structured fatal failures.
    ///
    /// This is the preferred entry point for editor integrations because an
    /// aborted load can still carry source anchors for the directive that made
    /// assembled-workspace facts unavailable.
    ///
    /// # Errors
    ///
    /// Returns a structured load error when the loader cannot normalize a root,
    /// traverse a workspace root, or read/follow a fatal local dependency.
    pub fn load_paths_with_failures<I, P>(
        &mut self,
        roots: I,
    ) -> WorkspaceLoadResult<WorkspaceFacts>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut normalized_roots = roots
            .into_iter()
            .map(|root| {
                let root = root.as_ref();
                normalize_existing_path(root).map_err(|error| {
                    WorkspaceLoadError::single(WorkspaceLoadFailure::workspace_root(root, &error))
                })
            })
            .collect::<WorkspaceLoadResult<Vec<_>>>()?;
        normalized_roots.sort();
        normalized_roots.dedup();

        WorkspaceBuildSession::new(&mut self.session).build_from_roots(&normalized_roots)
    }
}
// =============================================================================
// Private workspace-build session
// =============================================================================
//
// `WorkspaceLoader` remains the public compatibility facade, but the mutable
// state for one workspace load now lives in a dedicated session object. This is
// the next Salsa-oriented seam: future incremental workspace inputs and cached
// instance results can hang off this session without changing existing callers.

struct WorkspaceBuildSession<'loader> {
    session: &'loader mut WorkspaceAnalysisSession,
    loaded_documents: BTreeMap<PathBuf, WorkspaceDocument>,
    processed_contexts: BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    active_stack: Vec<PathBuf>,
}

impl<'loader> WorkspaceBuildSession<'loader> {
    const fn new(session: &'loader mut WorkspaceAnalysisSession) -> Self {
        Self {
            session,
            loaded_documents: BTreeMap::new(),
            processed_contexts: BTreeMap::new(),
            active_stack: Vec::new(),
        }
    }

    fn build_from_roots(
        mut self,
        normalized_roots: &[PathBuf],
    ) -> WorkspaceLoadResult<WorkspaceFacts> {
        // Phase 1: Normalize and scan the requested roots so broad workspace
        // discovery respects ignore rules before include traversal begins.
        self.scan_roots(normalized_roots)?;

        // Phase 2: Re-process the discovered documents in directive order so
        // constants, includes, and cycle detection follow the DSL's imperative
        // execution model.
        let start_contexts = self.process_start_contexts(normalized_roots)?;

        // Phase 3: Flatten the per-document include results into one stable
        // view for downstream diagnostics and editor features.
        Ok(self.finish(&start_contexts))
    }

    fn scan_roots(&mut self, normalized_roots: &[PathBuf]) -> WorkspaceLoadResult<()> {
        for root in normalized_roots {
            for path in scan_workspace_root(root).map_err(|error| {
                WorkspaceLoadError::single(WorkspaceLoadFailure::workspace_scan(root, &error))
            })? {
                self.load_document(path.clone(), true).map_err(|error| {
                    WorkspaceLoadError::single(WorkspaceLoadFailure::document_read(&path, &error))
                })?;
            }
        }

        Ok(())
    }

    fn process_start_contexts(
        &mut self,
        normalized_roots: &[PathBuf],
    ) -> WorkspaceLoadResult<Vec<DocumentContext>> {
        let start_contexts = start_contexts(normalized_roots, &self.loaded_documents);

        for context in &start_contexts {
            let _ = self.process_document_context(context.clone())?;
        }

        Ok(start_contexts)
    }

    fn finish(self, start_contexts: &[DocumentContext]) -> WorkspaceFacts {
        let workspace_indexes = build_workspace_indexes(
            self.session,
            &self.loaded_documents,
            start_contexts,
            &self.processed_contexts,
        );
        let assembly_key =
            workspace_facts_assembly_key(self.session, start_contexts, &self.processed_contexts);
        let assembly = build_workspace_facts_assembly(
            self.session,
            assembly_key,
            &self.processed_contexts,
            &workspace_indexes,
        );

        WorkspaceFacts {
            documents: self.loaded_documents.into_values().collect(),
            workspace_indexes,
            assembly,
        }
    }

    fn load_document(&mut self, path: PathBuf, discovered_by_scan: bool) -> io::Result<()> {
        if let Some(document) = self.loaded_documents.get_mut(&path) {
            if discovered_by_scan {
                document.mark_discovered_by_scan();
            }
            return Ok(());
        }

        let document = self.session.workspace_document(&path, discovered_by_scan)?;
        self.loaded_documents.insert(path, document);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn process_document_context(
        &mut self,
        context: DocumentContext,
    ) -> WorkspaceLoadResult<ConstantEnvironment> {
        // Memoize by `(path, inherited constants)` so repeated includes can
        // share the same processed result without rewalking the document.
        if let Some(processed_context) = self.processed_contexts.get(&context.key) {
            return Ok(processed_context.exported_constants.clone());
        }

        self.load_document(context.path.clone(), false)
            .map_err(|error| {
                WorkspaceLoadError::single(WorkspaceLoadFailure::document_read(
                    &context.path,
                    &error,
                ))
            })?;

        if self
            .cached_context_tree_is_fresh(&context.key, &mut BTreeSet::new())
            .unwrap_or(false)
            && self
                .materialize_cached_context_tree(&context.key, &mut BTreeSet::new())
                .is_ok()
        {
            return Ok(self
                .processed_contexts
                .get(&context.key)
                .expect("BUG: fresh cached context should materialize into the current load")
                .exported_constants
                .clone());
        }

        let (document_id, workspace_base, snapshot, directive_events) = {
            let document = self
                .loaded_documents
                .get(&context.path)
                .expect("BUG: document context should be loaded before processing");

            (
                document.id().clone(),
                workspace_base_directive(document.snapshot()),
                document.snapshot_handle(),
                document.directive_events_handle(),
            )
        };

        self.active_stack.push(context.path.clone());
        let processed = (|| -> WorkspaceLoadResult<ProcessedDocumentContext> {
            let mut current_constants = context.inherited_constants.clone();
            let mut workspace_base_context = None;
            let mut direct_includes = Vec::new();
            let mut included_contexts = Vec::new();

            // Load an extended base workspace before the current document's own
            // directives so inherited constants and bindings mirror the DSL's
            // top-down `workspace extends ...` semantics.
            if let Some(workspace_base) = &workspace_base {
                let resolved_base = resolve_workspace_base(
                    &document_id,
                    &context.path,
                    workspace_base,
                    &current_constants,
                )?;
                self.load_document(resolved_base.path.clone(), false)
                    .map_err(|error| {
                        let target_text = &resolved_base.target_text;
                        WorkspaceLoadError::single(WorkspaceLoadFailure::workspace_base(
                            &document_id,
                            workspace_base,
                            target_text,
                            Some(resolved_base.path.clone()),
                            format!("failed to load workspace base {target_text}: {error}"),
                        ))
                    })?;

                if self.active_stack.contains(&resolved_base.path) {
                    return Err(WorkspaceLoadError::single(
                        WorkspaceLoadFailure::workspace_base_cycle(
                            &document_id,
                            workspace_base,
                            &resolved_base.target_text,
                            resolved_base.path,
                        ),
                    ));
                }

                let child_context =
                    DocumentContext::new(resolved_base.path, current_constants.clone());
                workspace_base_context = Some(child_context.key.clone());
                current_constants = self.process_document_context(child_context)?;
            }

            // Process constants and includes in source order so inherited values,
            // local definitions, and included fragments all obey the DSL's
            // imperative execution model.
            for &event in directive_events.iter() {
                match event {
                    WorkspaceDirectiveEvent::ConstantDefinition(index) => {
                        let constant = snapshot
                            .constant_definitions()
                            .get(index)
                            .expect("BUG: cached directive event should point at a constant");
                        apply_constant_definition(constant, &mut current_constants);
                    }
                    WorkspaceDirectiveEvent::IncludeDirective(index) => {
                        let resolved_include = resolve_include(
                            &document_id,
                            &context.path,
                            snapshot
                                .include_directives()
                                .get(index)
                                .expect("BUG: cached directive event should point at an include"),
                            &current_constants,
                        )?;

                        for included_path in &resolved_include.discovered_paths {
                            self.load_document(included_path.clone(), false)
                                .map_err(|error| {
                                    WorkspaceLoadError::single(WorkspaceLoadFailure::include_load(
                                        resolved_include.include.including_document(),
                                        resolved_include.include.span(),
                                        resolved_include.include.value_span(),
                                        resolved_include.include.target_text(),
                                        Some(included_path.clone()),
                                        &error,
                                    ))
                                })?;
                        }

                        for included_path in &resolved_include.discovered_paths {
                            if self.active_stack.contains(included_path) {
                                continue;
                            }

                            let child_context = DocumentContext::new(
                                included_path.clone(),
                                current_constants.clone(),
                            );
                            included_contexts.push(child_context.key.clone());
                            current_constants = self.process_document_context(child_context)?;
                        }

                        direct_includes.push(resolved_include.include);
                    }
                }
            }

            Ok(ProcessedDocumentContext {
                exported_constants: current_constants,
                workspace_base_context,
                direct_includes,
                included_contexts,
            })
        })();
        let popped_path = self.active_stack.pop();
        debug_assert_eq!(popped_path.as_deref(), Some(context.path.as_path()));

        let processed = processed?;
        let exported_constants = processed.exported_constants.clone();
        self.session
            .store_processed_context(&context, processed.clone());
        self.processed_contexts.insert(context.key, processed);
        Ok(exported_constants)
    }

    fn cached_context_tree_is_fresh(
        &mut self,
        context_key: &DocumentContextKey,
        visiting: &mut BTreeSet<DocumentContextKey>,
    ) -> io::Result<bool> {
        if !visiting.insert(context_key.clone()) {
            return Ok(true);
        }

        let freshness = (|| -> io::Result<bool> {
            let Some(cached) = self.session.cached_processed_context(context_key) else {
                return Ok(false);
            };

            self.load_document(context_key.path.clone(), false)?;

            let Some(current_generation) = self.session.document_generation(&context_key.path)
            else {
                return Ok(false);
            };
            if current_generation != cached.document_generation {
                return Ok(false);
            }

            for (child_context, expected_revision) in
                processed_context_dependency_keys(&cached.processed)
                    .zip(&cached.child_context_revisions)
            {
                let Some(current_revision) = self.session.processed_context_revision(child_context)
                else {
                    return Ok(false);
                };
                if current_revision != *expected_revision {
                    return Ok(false);
                }
                if !self.cached_context_tree_is_fresh(child_context, visiting)? {
                    return Ok(false);
                }
            }

            for validation in &cached.include_validations {
                if !self.include_validation_is_fresh(validation)? {
                    return Ok(false);
                }
            }

            Ok(true)
        })();
        let _ = visiting.remove(context_key);
        freshness
    }

    fn materialize_cached_context_tree(
        &mut self,
        context_key: &DocumentContextKey,
        visiting: &mut BTreeSet<DocumentContextKey>,
    ) -> io::Result<()> {
        if self.processed_contexts.contains_key(context_key) {
            return Ok(());
        }
        if !visiting.insert(context_key.clone()) {
            return Ok(());
        }

        let materialized = (|| -> io::Result<()> {
            self.load_document(context_key.path.clone(), false)?;

            let cached = self
                .session
                .cached_processed_context(context_key)
                .expect("BUG: fresh cached context should still exist while materializing");

            for child_context in processed_context_dependency_keys(&cached.processed) {
                self.materialize_cached_context_tree(child_context, visiting)?;
            }

            self.processed_contexts
                .insert(context_key.clone(), cached.processed);
            Ok(())
        })();
        let _ = visiting.remove(context_key);
        materialized
    }

    fn include_validation_is_fresh(
        &mut self,
        validation: &CachedIncludeValidation,
    ) -> io::Result<bool> {
        match validation {
            CachedIncludeValidation::RemoteUrl { .. }
            | CachedIncludeValidation::UnsupportedLocalPath { .. } => Ok(true),
            CachedIncludeValidation::MissingLocalPath { path } => {
                Ok(fs::metadata(path).is_err_and(|error| error.kind() == io::ErrorKind::NotFound))
            }
            CachedIncludeValidation::LocalFile {
                path,
                document_generation,
            } => {
                self.load_document(path.clone(), false)?;
                Ok(self.session.document_generation(path) == Some(*document_generation))
            }
            CachedIncludeValidation::LocalDirectory {
                path,
                discovered_paths,
            } => {
                let allowed_root = path
                    .parent()
                    .expect("directory include path should have a parent");
                let current_paths = collect_directory_include_paths(path, allowed_root)?;
                if current_paths.len() != discovered_paths.len() {
                    return Ok(false);
                }

                for (current_path, (expected_path, expected_generation)) in
                    current_paths.iter().zip(discovered_paths)
                {
                    if current_path != expected_path {
                        return Ok(false);
                    }

                    self.load_document(current_path.clone(), false)?;
                    if self.session.document_generation(current_path) != Some(*expected_generation)
                    {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod workspace_loader_tests {
    use super::*;

    use indoc::indoc;
    use std::ptr;

    #[test]
    fn loader_reuses_cached_document_snapshots_across_identical_loads() {
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
        let second = loader
            .load_paths([fixture.root()])
            .expect("second load should succeed");

        let first_workspace = first
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        let second_workspace = second
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        assert!(ptr::eq(
            first_workspace.snapshot(),
            second_workspace.snapshot()
        ));

        let first_model = first
            .document(&document_id_from_path(&fixture.model_path()))
            .expect("included model document should exist");
        let second_model = second
            .document(&document_id_from_path(&fixture.model_path()))
            .expect("included model document should exist");
        assert!(ptr::eq(first_model.snapshot(), second_model.snapshot()));
    }

    #[test]
    fn loader_refreshes_cached_snapshot_when_override_changes() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                model {
                    user = person "User"
                }
            }
        "#});

        let mut loader = WorkspaceLoader::new();
        let first = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("first load should succeed");

        loader.set_document_override(
            fixture.workspace_path().clone(),
            indoc! {r#"
                workspace {
                    model {
                        admin = person "Admin"
                    }
                }
            "#}
            .to_owned(),
        );

        let second = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("override-backed load should succeed");

        let first_document = first
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        let second_document = second
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");

        assert!(!ptr::eq(
            first_document.snapshot(),
            second_document.snapshot()
        ));
        assert_eq!(
            second_document
                .snapshot()
                .symbols()
                .iter()
                .filter_map(|symbol| symbol.binding_name.as_deref())
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
    }

    #[test]
    fn clearing_document_overrides_restores_disk_backed_snapshot() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                model {
                    user = person "User"
                }
            }
        "#});

        let mut loader = WorkspaceLoader::new();
        loader.set_document_override(
            fixture.workspace_path().clone(),
            indoc! {r#"
                workspace {
                    model {
                        admin = person "Admin"
                    }
                }
            "#}
            .to_owned(),
        );
        let override_backed = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("override-backed load should succeed");

        loader.clear_document_overrides();
        let disk_backed = loader
            .load_paths([fixture.workspace_path().as_path()])
            .expect("disk-backed reload should succeed");

        let override_document = override_backed
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");
        let disk_document = disk_backed
            .document(&document_id_from_path(fixture.workspace_path()))
            .expect("workspace document should exist");

        assert_eq!(
            override_document
                .snapshot()
                .symbols()
                .iter()
                .filter_map(|symbol| symbol.binding_name.as_deref())
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
        assert_eq!(
            disk_document
                .snapshot()
                .symbols()
                .iter()
                .filter_map(|symbol| symbol.binding_name.as_deref())
                .collect::<Vec<_>>(),
            vec!["user"]
        );
    }
}
