// Workspace-instance and index assembly, including document collection and the
// final semantic packets exposed to downstream callers.

/// Effective element-identifier mode for one document inside one workspace instance.
///
/// This folds together the document-local `!identifiers` directives and any
/// inherited workspace-level mode so downstream consumers do not need to
/// re-derive the same policy from raw directive facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementIdentifierMode {
    /// Element bindings resolve through their flat binding names.
    ///
    /// Example: with `!identifiers flat`, a container declared as
    /// `api = container "API"` is referenced as `api`.
    Flat,
    /// Element bindings resolve through canonical hierarchical keys.
    ///
    /// Example: with `!identifiers hierarchical`, a container declared as
    /// `api = container "API"` inside `softwareSystem1 = softwareSystem "System 1"`
    /// is referenced as `softwareSystem1.api`.
    Hierarchical,
    /// Element bindings stay intentionally deferred because the effective mode is
    /// unsupported for the bounded semantic surface.
    ///
    /// Example: `!identifiers custom` is parsed as an unrecognized mode, so the
    /// workspace index records the document as deferred instead of guessing how
    /// a reference such as `api` should resolve.
    Deferred,
}

fn build_workspace_indexes(
    session: &mut WorkspaceAnalysisSession,
    loaded_documents: &BTreeMap<PathBuf, WorkspaceDocument>,
    start_contexts: &[DocumentContext],
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
) -> Vec<WorkspaceIndex> {
    let documents_by_id = loaded_documents
        .values()
        .map(|document| (document.id().clone(), document))
        .collect::<BTreeMap<_, _>>();

    start_contexts
        .iter()
        .enumerate()
        .map(|(ordinal, start_context)| {
            let instance_id = WorkspaceInstanceId(ordinal);
            build_workspace_index(
                session,
                instance_id,
                start_context,
                processed_contexts,
                &documents_by_id,
            )
        })
        .collect()
}

fn build_document_instances(
    workspace_indexes: &[WorkspaceIndex],
) -> BTreeMap<DocumentId, Vec<WorkspaceInstanceId>> {
    let mut document_instances = BTreeMap::<DocumentId, Vec<WorkspaceInstanceId>>::new();

    for workspace_index in workspace_indexes {
        for document in workspace_index.documents() {
            document_instances
                .entry(document.clone())
                .or_default()
                .push(workspace_index.id());
        }
    }

    document_instances
}

fn build_workspace_index(
    session: &mut WorkspaceAnalysisSession,
    instance_id: WorkspaceInstanceId,
    start_context: &DocumentContext,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
) -> WorkspaceIndex {
    let _root_context_revision = session
        .processed_context_revision(&start_context.key)
        .expect("BUG: start context should be processed before building indexes");

    // Build two slices from the same processed-context tree:
    //
    // 1. the full assembled instance, which includes extended bases because
    //    binding/reference resolution needs the final inherited symbol table;
    // 2. the narrower DSL definition, which intentionally excludes bases so
    //    structural rules like repeated `model` / `views` sections talk about
    //    one definition rather than one definition plus its parent workspace.
    let instance_documents = collect_documents_for_context(
        &start_context.key,
        processed_contexts,
        ContextDocumentCollection::Instance,
    );
    let definition_documents = collect_documents_for_context(
        &start_context.key,
        processed_contexts,
        ContextDocumentCollection::Definition,
    );
    let inherited_workspace_modes = inherited_workspace_modes_for_context(
        &start_context.key,
        processed_contexts,
        documents_by_id,
    );
    let document_semantic_generations = instance_documents
        .iter()
        .map(|document_id| {
            let document = documents_by_id
                .get(document_id)
                .expect("BUG: workspace-index document should exist");
            (document_id.clone(), document.semantic_generation())
        })
        .collect::<Vec<_>>();

    if let Some(cached) = session.cached_workspace_instance(&start_context.key)
        && cached.document_semantic_generations == document_semantic_generations
    {
        return cached.workspace_index(instance_id);
    }

    let root_document = document_id_from_path(&start_context.path);
    let root_document = documents_by_id
        .get(&root_document)
        .expect("BUG: workspace-index root document should exist");
    let instance_semantic_documents = instance_documents
        .iter()
        .map(|document_id| {
            documents_by_id
                .get(document_id)
                .expect("BUG: workspace-index document should exist")
                .semantic_facts()
        })
        .collect::<Vec<_>>();
    let definition_semantic_documents = definition_documents
        .iter()
        .map(|document_id| {
            documents_by_id
                .get(document_id)
                .expect("BUG: workspace-index definition document should exist")
                .semantic_facts()
        })
        .collect::<Vec<_>>();
    let derived = Arc::new(build_derived_workspace_instance(
        root_document.semantic_facts(),
        &instance_semantic_documents,
        &definition_semantic_documents,
        &inherited_workspace_modes,
    ));
    session.store_workspace_instance(
        &start_context.key,
        document_semantic_generations,
        Arc::clone(&derived),
    );
    WorkspaceIndex::from_derived(instance_id, derived)
}

fn workspace_facts_assembly_key(
    session: &WorkspaceAnalysisSession,
    start_contexts: &[DocumentContext],
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
) -> WorkspaceFactsAssemblyKey {
    let workspace_instances = start_contexts
        .iter()
        .map(|start_context| RevisionedContextKey {
            context_key: start_context.key.clone(),
            revision: session
                .processed_context_revision(&start_context.key)
                .expect("BUG: start context should be processed before assembly"),
        })
        .collect();

    let processed_contexts = processed_contexts
        .keys()
        .map(|context_key| RevisionedContextKey {
            context_key: context_key.clone(),
            revision: session
                .processed_context_revision(context_key)
                .expect("BUG: materialized processed context should have a cached revision"),
        })
        .collect();

    WorkspaceFactsAssemblyKey {
        processed_contexts,
        workspace_instances,
    }
}

fn build_workspace_facts_assembly(
    session: &mut WorkspaceAnalysisSession,
    assembly_key: WorkspaceFactsAssemblyKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    workspace_indexes: &[WorkspaceIndex],
) -> Arc<DerivedWorkspaceFactsAssembly> {
    if let Some(cached) = session.cached_workspace_facts_assembly(&assembly_key) {
        return cached;
    }

    let mut resolved_includes = processed_contexts
        .values()
        .flat_map(|context| context.direct_includes.iter().cloned())
        .collect::<Vec<_>>();
    resolved_includes.sort_by(|left, right| {
        left.including_document()
            .as_str()
            .cmp(right.including_document().as_str())
            .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
            .then_with(|| left.target_text().cmp(right.target_text()))
    });

    let include_diagnostics = include_diagnostics(&resolved_includes);
    let document_instances = build_document_instances(workspace_indexes);
    let semantic_diagnostics = merge_semantic_diagnostics(workspace_indexes, &document_instances);

    let assembly = Arc::new(DerivedWorkspaceFactsAssembly {
        resolved_includes,
        include_diagnostics,
        document_instances,
        semantic_diagnostics,
    });
    session.store_workspace_facts_assembly(assembly_key, Arc::clone(&assembly));
    assembly
}

fn build_derived_workspace_instance(
    root_document: &WorkspaceSemanticDocumentFacts,
    documents: &[&WorkspaceSemanticDocumentFacts],
    definition_documents: &[&WorkspaceSemanticDocumentFacts],
    inherited_workspace_modes: &BTreeMap<DocumentId, Option<IdentifierMode>>,
) -> DerivedWorkspaceInstance {
    // `definition_documents` exists purely to scope structural workspace rules to
    // one assembled definition. We intentionally keep only the full instance
    // document list on the derived payload because downstream callers reason
    // about the assembled semantic surface, not the narrower structural slice.
    let bindings = build_binding_tables(documents, inherited_workspace_modes);
    let workspace_symbols = build_workspace_symbols(&root_document.document_id, &bindings);
    let mut semantic_diagnostics = bindings.semantic_diagnostics.clone();
    semantic_diagnostics.extend(workspace_structure_diagnostics(definition_documents));
    semantic_diagnostics.extend(workspace_scope_diagnostics(definition_documents, documents));
    semantic_diagnostics.extend(element_selector_diagnostics(documents, &bindings));

    let reference_tables = build_reference_resolution_tables(documents, &bindings);
    let documents_by_id = documents
        .iter()
        .map(|document| (document.document_id.clone(), *document))
        .collect::<BTreeMap<_, _>>();
    semantic_diagnostics.extend(deployment_semantic_diagnostics(
        documents,
        &documents_by_id,
        &bindings,
        &reference_tables,
    ));
    semantic_diagnostics.extend(resource_semantic_diagnostics(documents));
    semantic_diagnostics.extend(view_semantic_diagnostics(
        documents,
        &documents_by_id,
        &bindings,
        &reference_tables,
    ));
    semantic_diagnostics.extend(reference_tables.semantic_diagnostics);

    let mut references_by_target = reference_tables.references_by_target;
    for references in references_by_target.values_mut() {
        references.sort();
        references.dedup();
    }
    sort_semantic_diagnostics(&mut semantic_diagnostics);

    DerivedWorkspaceInstance {
        root_document: root_document.document_id.clone(),
        documents: documents
            .iter()
            .map(|document| document.document_id.clone())
            .collect(),
        element_identifier_modes: bindings.element_modes,
        unique_element_bindings: bindings.unique_elements,
        duplicate_element_bindings: bindings.duplicate_elements,
        unique_deployment_bindings: bindings.unique_deployments,
        duplicate_deployment_bindings: bindings.duplicate_deployments,
        unique_relationship_bindings: bindings.unique_relationships,
        duplicate_relationship_bindings: bindings.duplicate_relationships,
        workspace_symbols,
        reference_resolutions: reference_tables.resolutions,
        references_by_target,
        semantic_diagnostics,
    }
}

/// The same processed-context tree feeds two different semantic projections.
///
/// `Instance` walks the complete assembled workspace, including extended bases,
/// because binding/reference resolution needs the final inherited symbol table.
/// `Definition` stops at ordinary includes so structural rules can talk about
/// one DSL definition without treating its extended base as a repeated sibling.
#[derive(Clone, Copy)]
enum ContextDocumentCollection {
    Instance,
    Definition,
}

fn collect_documents_for_context(
    context_key: &DocumentContextKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    collection: ContextDocumentCollection,
) -> Vec<DocumentId> {
    let mut visited_contexts = BTreeSet::new();
    let mut seen_documents = BTreeSet::new();
    let mut collected_documents = Vec::new();
    collect_context_documents(
        context_key,
        processed_contexts,
        collection,
        &mut visited_contexts,
        &mut seen_documents,
        &mut collected_documents,
    );
    collected_documents
}

fn collect_context_documents(
    context_key: &DocumentContextKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    collection: ContextDocumentCollection,
    visited_contexts: &mut BTreeSet<DocumentContextKey>,
    seen_documents: &mut BTreeSet<DocumentId>,
    collected_documents: &mut Vec<DocumentId>,
) {
    if !visited_contexts.insert(context_key.clone()) {
        return;
    }

    let document_id = document_id_from_path(&context_key.path);
    if seen_documents.insert(document_id.clone()) {
        collected_documents.push(document_id);
    }

    let Some(processed_context) = processed_contexts.get(context_key) else {
        return;
    };

    match collection {
        ContextDocumentCollection::Instance => {
            for child_context in processed_context_dependency_keys(processed_context) {
                collect_context_documents(
                    child_context,
                    processed_contexts,
                    collection,
                    visited_contexts,
                    seen_documents,
                    collected_documents,
                );
            }
        }
        ContextDocumentCollection::Definition => {
            for child_context in &processed_context.included_contexts {
                collect_context_documents(
                    child_context,
                    processed_contexts,
                    collection,
                    visited_contexts,
                    seen_documents,
                    collected_documents,
                );
            }
        }
    }
}

fn inherited_workspace_modes_for_context(
    context_key: &DocumentContextKey,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
) -> BTreeMap<DocumentId, Option<IdentifierMode>> {
    let mut visited_contexts = BTreeSet::new();
    let mut inherited_modes = BTreeMap::new();
    collect_inherited_workspace_modes(
        context_key,
        None,
        processed_contexts,
        documents_by_id,
        &mut visited_contexts,
        &mut inherited_modes,
    );
    inherited_modes
}

fn collect_inherited_workspace_modes(
    context_key: &DocumentContextKey,
    inherited_workspace_mode: Option<IdentifierMode>,
    processed_contexts: &BTreeMap<DocumentContextKey, ProcessedDocumentContext>,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceDocument>,
    visited_contexts: &mut BTreeSet<DocumentContextKey>,
    inherited_modes: &mut BTreeMap<DocumentId, Option<IdentifierMode>>,
) {
    if !visited_contexts.insert(context_key.clone()) {
        return;
    }

    let document_id = document_id_from_path(&context_key.path);
    if let Some(existing_mode) = inherited_modes.get(&document_id) {
        debug_assert_eq!(existing_mode, &inherited_workspace_mode);
    } else {
        inherited_modes.insert(document_id.clone(), inherited_workspace_mode.clone());
    }

    let Some(document) = documents_by_id.get(&document_id) else {
        return;
    };
    let next_workspace_mode =
        document_workspace_identifier_mode(&document.semantic_facts().identifier_modes)
            .or(inherited_workspace_mode);
    let Some(processed_context) = processed_contexts.get(context_key) else {
        return;
    };

    for child_context in processed_context_dependency_keys(processed_context) {
        collect_inherited_workspace_modes(
            child_context,
            next_workspace_mode.clone(),
            processed_contexts,
            documents_by_id,
            visited_contexts,
            inherited_modes,
        );
    }
}

struct WorkspaceBindingTables {
    element_modes: BTreeMap<DocumentId, ElementIdentifierMode>,
    unique_elements: BTreeMap<String, SymbolHandle>,
    duplicate_elements: BTreeMap<String, Vec<SymbolHandle>>,
    unique_deployments: BTreeMap<String, SymbolHandle>,
    duplicate_deployments: BTreeMap<String, Vec<SymbolHandle>>,
    unique_relationships: BTreeMap<String, SymbolHandle>,
    duplicate_relationships: BTreeMap<String, Vec<SymbolHandle>>,
    semantic_diagnostics: Vec<RuledDiagnostic>,
}

fn build_workspace_symbols(
    root_document: &DocumentId,
    bindings: &WorkspaceBindingTables,
) -> Vec<WorkspaceSymbolFact> {
    let unique_root_document = root_document.clone();
    let duplicate_root_document = root_document.clone();

    // Keep the assembly order explicit here: project every unique binding first,
    // then expand the duplicate sets that fan out to multiple declarations.
    let mut symbols = [
        &bindings.unique_elements,
        &bindings.unique_deployments,
        &bindings.unique_relationships,
    ]
    .into_iter()
    .flat_map(|bindings| {
        bindings.iter().map(|(canonical_key, handle)| {
            WorkspaceSymbolFact::new(
                canonical_key.clone(),
                unique_root_document.clone(),
                handle.document().clone(),
                handle.clone(),
            )
        })
    })
    .chain(
        [
            &bindings.duplicate_elements,
            &bindings.duplicate_deployments,
            &bindings.duplicate_relationships,
        ]
        .into_iter()
        .flat_map(|bindings| {
            bindings.iter().flat_map(|(canonical_key, handles)| {
                let duplicate_root_document = duplicate_root_document.clone();
                handles.iter().map(move |handle| {
                    WorkspaceSymbolFact::new(
                        canonical_key.clone(),
                        duplicate_root_document.clone(),
                        handle.document().clone(),
                        handle.clone(),
                    )
                })
            })
        }),
    )
    .collect::<Vec<_>>();

    symbols.sort();
    symbols
}

fn build_binding_tables(
    documents: &[&WorkspaceSemanticDocumentFacts],
    inherited_workspace_modes: &BTreeMap<DocumentId, Option<IdentifierMode>>,
) -> WorkspaceBindingTables {
    let mut element_modes = BTreeMap::<DocumentId, ElementIdentifierMode>::new();
    let mut element_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();
    let mut deployment_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();
    let mut relationship_bindings = BTreeMap::<String, Vec<SymbolHandle>>::new();

    for document in documents {
        let inherited_workspace_mode = inherited_workspace_modes
            .get(&document.document_id)
            .and_then(|mode| mode.as_ref());
        let element_mode = effective_element_identifier_mode(document, inherited_workspace_mode);
        element_modes.insert(document.document_id.clone(), element_mode);

        for symbol in &document.symbols {
            let Some(binding_name) = symbol.binding_name.as_deref() else {
                continue;
            };

            let handle = SymbolHandle {
                document: document.document_id.clone(),
                symbol_id: symbol.id,
            };

            match symbol.kind {
                SymbolKind::Relationship => {
                    relationship_bindings
                        .entry(binding_name.to_owned())
                        .or_default()
                        .push(handle);
                }
                SymbolKind::Person
                | SymbolKind::SoftwareSystem
                | SymbolKind::Container
                | SymbolKind::Component => {
                    let Some(binding_key) = canonical_binding_key(
                        &document.symbols,
                        symbol.id,
                        element_mode,
                        CanonicalBindingKind::Element,
                    ) else {
                        continue;
                    };

                    element_bindings
                        .entry(binding_key)
                        .or_default()
                        .push(handle);
                }
                SymbolKind::DeploymentEnvironment
                | SymbolKind::DeploymentNode
                | SymbolKind::InfrastructureNode
                | SymbolKind::ContainerInstance
                | SymbolKind::SoftwareSystemInstance => {
                    let Some(binding_key) = canonical_binding_key(
                        &document.symbols,
                        symbol.id,
                        element_mode,
                        CanonicalBindingKind::Deployment,
                    ) else {
                        continue;
                    };

                    deployment_bindings
                        .entry(binding_key)
                        .or_default()
                        .push(handle);
                }
            }
        }
    }

    let (unique_element_bindings, duplicate_element_bindings) =
        split_binding_table(element_bindings);
    let (unique_deployment_bindings, duplicate_deployment_bindings) =
        split_binding_table(deployment_bindings);
    let (unique_relationship_bindings, duplicate_relationship_bindings) =
        split_binding_table(relationship_bindings);

    let mut semantic_diagnostics = Vec::new();
    push_duplicate_binding_diagnostics(
        "element",
        &duplicate_element_bindings,
        documents,
        &mut semantic_diagnostics,
    );
    push_duplicate_binding_diagnostics(
        "deployment",
        &duplicate_deployment_bindings,
        documents,
        &mut semantic_diagnostics,
    );
    push_duplicate_binding_diagnostics(
        "relationship",
        &duplicate_relationship_bindings,
        documents,
        &mut semantic_diagnostics,
    );

    WorkspaceBindingTables {
        element_modes,
        unique_elements: unique_element_bindings,
        duplicate_elements: duplicate_element_bindings,
        unique_deployments: unique_deployment_bindings,
        duplicate_deployments: duplicate_deployment_bindings,
        unique_relationships: unique_relationship_bindings,
        duplicate_relationships: duplicate_relationship_bindings,
        semantic_diagnostics,
    }
}

struct WorkspaceReferenceTables {
    resolutions: BTreeMap<ReferenceHandle, ReferenceResolutionStatus>,
    // Later semantic passes ask "what symbol does this exact `(kind, span)` site
    // resolve to?" far more often than they ask for the raw resolution enum.
    // Cache that direct lookup once so view/deployment rules do not repeatedly
    // rescan `document.references` just to rediscover the same target handle.
    resolved_targets: BTreeMap<DocumentId, BTreeMap<(u8, TextSpan), SymbolHandle>>,
    references_by_target: BTreeMap<SymbolHandle, Vec<ReferenceHandle>>,
    semantic_diagnostics: Vec<RuledDiagnostic>,
}

fn build_reference_resolution_tables(
    documents: &[&WorkspaceSemanticDocumentFacts],
    bindings: &WorkspaceBindingTables,
) -> WorkspaceReferenceTables {
    let documents_by_id = documents
        .iter()
        .map(|document| (document.document_id.clone(), *document))
        .collect::<BTreeMap<_, _>>();
    let mut reference_resolutions = BTreeMap::<ReferenceHandle, ReferenceResolutionStatus>::new();
    let mut resolved_targets =
        BTreeMap::<DocumentId, BTreeMap<(u8, TextSpan), SymbolHandle>>::new();
    let mut references_by_target = BTreeMap::<SymbolHandle, Vec<ReferenceHandle>>::new();
    let mut semantic_diagnostics = Vec::new();

    for document in documents {
        for (reference_index, reference) in document.references.iter().enumerate() {
            let handle = ReferenceHandle {
                document: document.document_id.clone(),
                reference_index,
            };
            let status = resolve_reference_status(document, reference, bindings, &documents_by_id);

            if !document.has_syntax_errors {
                match status {
                    ReferenceResolutionStatus::UnresolvedNoMatch => {
                        if emits_generic_reference_diagnostics(reference) {
                            semantic_diagnostics.push(unresolved_reference_diagnostic(
                                &document.document_id,
                                reference,
                            ));
                        }
                    }
                    ReferenceResolutionStatus::AmbiguousDuplicateBinding
                    | ReferenceResolutionStatus::AmbiguousElementVsRelationship => {
                        if emits_generic_reference_diagnostics(reference) {
                            semantic_diagnostics.push(ambiguous_reference_diagnostic(
                                &document.document_id,
                                reference,
                                &status,
                            ));
                        }
                    }
                    ReferenceResolutionStatus::Resolved(_)
                    | ReferenceResolutionStatus::DeferredByScopePolicy => {}
                }
            }

            if let ReferenceResolutionStatus::Resolved(target) = &status {
                resolved_targets
                    .entry(document.document_id.clone())
                    .or_default()
                    .insert(
                        reference_lookup_key(reference.kind, reference.span),
                        target.clone(),
                    );
                references_by_target
                    .entry(target.clone())
                    .or_default()
                    .push(handle.clone());
            }

            reference_resolutions.insert(handle, status);
        }
    }

    WorkspaceReferenceTables {
        resolutions: reference_resolutions,
        resolved_targets,
        references_by_target,
        semantic_diagnostics,
    }
}
