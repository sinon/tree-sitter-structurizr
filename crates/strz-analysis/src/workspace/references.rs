// Relationship extraction joins, contextual owner lookup, and workspace-wide
// reference resolution helpers built on top of the binding tables.

fn collect_declared_relationships(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<DeclaredRelationship> {
    let mut relationships = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        // Relationship facts already carry spans, while the symbol table owns the
        // stable symbol IDs/handles. Index the relationship symbols once per
        // document so later collection can join those two views of the same
        // declaration without rescanning the whole symbol list for every fact.
        let relationship_handles = document
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Relationship)
            .map(|symbol| {
                (
                    symbol.span,
                    SymbolHandle::new(document.document_id.clone(), symbol.id),
                )
            })
            .collect::<BTreeMap<_, _>>();

        for relationship in &document.relationship_facts {
            let Some(source) = resolved_declared_relationship_endpoint(
                document,
                relationship.source.as_ref(),
                relationship.span,
                ReferenceKind::RelationshipSource,
                ReferenceKind::DeploymentRelationshipSource,
                bindings,
                reference_tables,
            ) else {
                continue;
            };
            let Some(destination) = resolved_declared_relationship_endpoint(
                document,
                Some(&relationship.destination),
                relationship.span,
                ReferenceKind::RelationshipDestination,
                ReferenceKind::DeploymentRelationshipDestination,
                bindings,
                reference_tables,
            ) else {
                continue;
            };
            let Some(source_symbol) = symbol_for_handle(documents_by_id, &source) else {
                continue;
            };
            let Some(destination_symbol) = symbol_for_handle(documents_by_id, &destination) else {
                continue;
            };
            if !is_model_element_kind(source_symbol.kind)
                || !is_model_element_kind(destination_symbol.kind)
            {
                continue;
            }

            relationships.push(DeclaredRelationship {
                handle: relationship_handles.get(&relationship.span).cloned(),
                document: document.document_id.clone(),
                span: relationship.span,
                source,
                destination,
                technology: relationship
                    .technology
                    .as_ref()
                    .map(|value| value.normalized_text.clone()),
            });
        }
    }

    relationships
}

fn resolved_relationship_target(
    document: &WorkspaceSemanticDocumentFacts,
    primary_kind: ReferenceKind,
    fallback_kind: ReferenceKind,
    span: TextSpan,
    reference_tables: &WorkspaceReferenceTables,
) -> Option<SymbolHandle> {
    resolved_reference_target(document, primary_kind, span, reference_tables)
        .or_else(|| resolved_reference_target(document, fallback_kind, span, reference_tables))
}

fn resolved_declared_relationship_endpoint(
    document: &WorkspaceSemanticDocumentFacts,
    endpoint: Option<&ValueFact>,
    relationship_span: TextSpan,
    primary_kind: ReferenceKind,
    fallback_kind: ReferenceKind,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
) -> Option<SymbolHandle> {
    endpoint.map_or_else(
        || {
            contextual_owner_resolution(
                document,
                relationship_span,
                enclosing_symbol_for_span(document, relationship_span),
                ContextualOwnerTarget::ElementOrDeployment,
                bindings,
            )
            .resolved()
        },
        |value| {
            resolved_relationship_target(
                document,
                primary_kind,
                fallback_kind,
                value.span,
                reference_tables,
            )
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextualOwnerTarget {
    Element,
    Deployment,
    ElementOrDeployment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ContextualOwnerResolution {
    Resolved(SymbolHandle),
    Unresolved,
    Ambiguous,
}

impl ContextualOwnerResolution {
    fn resolved(self) -> Option<SymbolHandle> {
        match self {
            Self::Resolved(handle) => Some(handle),
            Self::Unresolved | Self::Ambiguous => None,
        }
    }
}

fn contextual_owner_resolution(
    document: &WorkspaceSemanticDocumentFacts,
    span: TextSpan,
    start_symbol: Option<SymbolId>,
    target: ContextualOwnerTarget,
    bindings: &WorkspaceBindingTables,
) -> ContextualOwnerResolution {
    if let Some(selector_resolution) =
        enclosing_element_selector_owner_resolution(document, span, target, bindings)
    {
        return selector_resolution;
    }

    contextual_symbol_target_handle_for_owner(document, start_symbol, target).map_or(
        ContextualOwnerResolution::Unresolved,
        ContextualOwnerResolution::Resolved,
    )
}

fn enclosing_element_selector_owner_resolution(
    document: &WorkspaceSemanticDocumentFacts,
    span: TextSpan,
    target: ContextualOwnerTarget,
    bindings: &WorkspaceBindingTables,
) -> Option<ContextualOwnerResolution> {
    let directive = enclosing_element_directive(document, span)?;
    Some(resolve_element_selector_owner_resolution(
        document, directive, target, bindings,
    ))
}

fn resolve_element_selector_owner_resolution(
    document: &WorkspaceSemanticDocumentFacts,
    directive: &ElementDirectiveFact,
    target: ContextualOwnerTarget,
    bindings: &WorkspaceBindingTables,
) -> ContextualOwnerResolution {
    if !matches!(
        directive.target.value_kind,
        DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
    ) {
        return ContextualOwnerResolution::Unresolved;
    }

    for candidate in element_selector_target_candidates(document, directive, bindings) {
        let resolution = resolve_contextual_selector_candidate(&candidate, target, bindings);
        if resolution != ContextualOwnerResolution::Unresolved {
            return resolution;
        }
    }

    ContextualOwnerResolution::Unresolved
}

fn resolve_contextual_selector_candidate(
    candidate: &str,
    target: ContextualOwnerTarget,
    bindings: &WorkspaceBindingTables,
) -> ContextualOwnerResolution {
    match target {
        ContextualOwnerTarget::Element => {
            match resolve_reference_against_element_table(
                candidate,
                &bindings.unique_elements,
                &bindings.duplicate_elements,
            ) {
                ReferenceResolutionStatus::Resolved(handle) => {
                    ContextualOwnerResolution::Resolved(handle)
                }
                ReferenceResolutionStatus::AmbiguousDuplicateBinding
                | ReferenceResolutionStatus::AmbiguousElementVsRelationship => {
                    ContextualOwnerResolution::Ambiguous
                }
                ReferenceResolutionStatus::UnresolvedNoMatch
                | ReferenceResolutionStatus::DeferredByScopePolicy => {
                    ContextualOwnerResolution::Unresolved
                }
            }
        }
        ContextualOwnerTarget::Deployment => {
            match resolve_reference_against_binding_table(
                candidate,
                &bindings.unique_deployments,
                &bindings.duplicate_deployments,
            ) {
                ReferenceResolutionStatus::Resolved(handle) => {
                    ContextualOwnerResolution::Resolved(handle)
                }
                ReferenceResolutionStatus::AmbiguousDuplicateBinding
                | ReferenceResolutionStatus::AmbiguousElementVsRelationship => {
                    ContextualOwnerResolution::Ambiguous
                }
                ReferenceResolutionStatus::UnresolvedNoMatch
                | ReferenceResolutionStatus::DeferredByScopePolicy => {
                    ContextualOwnerResolution::Unresolved
                }
            }
        }
        ContextualOwnerTarget::ElementOrDeployment => {
            match resolve_selector_target_raw_text(candidate, bindings) {
                SelectorResolutionStatus::Resolved => bindings
                    .unique_elements
                    .get(candidate)
                    .or_else(|| bindings.unique_deployments.get(candidate))
                    .cloned()
                    .map_or(
                        ContextualOwnerResolution::Unresolved,
                        ContextualOwnerResolution::Resolved,
                    ),
                SelectorResolutionStatus::Ambiguous => ContextualOwnerResolution::Ambiguous,
                SelectorResolutionStatus::UnresolvedNoMatch => {
                    ContextualOwnerResolution::Unresolved
                }
            }
        }
    }
}

fn resolved_reference_target(
    document: &WorkspaceSemanticDocumentFacts,
    reference_kind: ReferenceKind,
    span: TextSpan,
    reference_tables: &WorkspaceReferenceTables,
) -> Option<SymbolHandle> {
    // Semantic rules talk about source spans and reference roles, not raw
    // reference indices. Route everything through the prebuilt lookup so callers
    // do not need to know how the reference table is stored internally.
    reference_tables
        .resolved_targets
        .get(&document.document_id)?
        .get(&reference_lookup_key(reference_kind, span))
        .cloned()
}

const fn reference_lookup_key(reference_kind: ReferenceKind, span: TextSpan) -> (u8, TextSpan) {
    (reference_kind_index(reference_kind), span)
}

const fn reference_kind_index(reference_kind: ReferenceKind) -> u8 {
    match reference_kind {
        ReferenceKind::ElementSelectorTarget => 0,
        ReferenceKind::RelationshipSource => 1,
        ReferenceKind::RelationshipDestination => 2,
        ReferenceKind::DynamicRelationshipReference => 3,
        ReferenceKind::InstanceTarget => 4,
        ReferenceKind::DeploymentRelationshipSource => 5,
        ReferenceKind::DeploymentRelationshipDestination => 6,
        ReferenceKind::ViewScope => 7,
        ReferenceKind::ViewInclude => 8,
        ReferenceKind::ViewExclude => 9,
        ReferenceKind::ViewAnimation => 10,
    }
}

fn symbol_for_handle<'a>(
    documents_by_id: &'a BTreeMap<DocumentId, &'a WorkspaceSemanticDocumentFacts>,
    handle: &SymbolHandle,
) -> Option<&'a Symbol> {
    documents_by_id
        .get(handle.document())
        .and_then(|document| document.symbols.get(handle.symbol_id().0))
}

const fn is_model_element_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Person
            | SymbolKind::SoftwareSystem
            | SymbolKind::Container
            | SymbolKind::Component
    )
}

const fn is_deployment_element_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::DeploymentEnvironment
            | SymbolKind::DeploymentNode
            | SymbolKind::InfrastructureNode
            | SymbolKind::ContainerInstance
            | SymbolKind::SoftwareSystemInstance
    )
}

/// Encodes the current upstream parity boundary for view families whose include
/// and animation members we validate semantically.
///
/// Keeping the matrix centralized makes later view-rule slices read in terms of
/// "which elements may this view family show?" rather than scattering that policy
/// across individual diagnostics.
const fn is_view_element_kind_allowed(view_kind: ViewKind, symbol_kind: SymbolKind) -> bool {
    match view_kind {
        ViewKind::SystemLandscape | ViewKind::SystemContext => {
            matches!(symbol_kind, SymbolKind::Person | SymbolKind::SoftwareSystem)
        }
        ViewKind::Container => matches!(
            symbol_kind,
            SymbolKind::Person | SymbolKind::SoftwareSystem | SymbolKind::Container
        ),
        ViewKind::Component => matches!(
            symbol_kind,
            SymbolKind::Person
                | SymbolKind::SoftwareSystem
                | SymbolKind::Container
                | SymbolKind::Component
        ),
        ViewKind::Filtered
        | ViewKind::Dynamic
        | ViewKind::Deployment
        | ViewKind::Custom
        | ViewKind::Image => false,
    }
}

fn matching_declared_relationship<'a>(
    source: &SymbolHandle,
    destination: &SymbolHandle,
    technology: Option<&str>,
    declared_relationships: &'a [DeclaredRelationship],
) -> Option<&'a DeclaredRelationship> {
    declared_relationships.iter().find(|relationship| {
        relationship.source == *source
            && relationship.destination == *destination
            && relationship_technology_matches(relationship.technology.as_deref(), technology)
    })
}

fn declared_relationship_for_handle<'a>(
    handle: &SymbolHandle,
    declared_relationships: &'a [DeclaredRelationship],
) -> Option<&'a DeclaredRelationship> {
    declared_relationships
        .iter()
        .find(|relationship| relationship.handle.as_ref() == Some(handle))
}

fn deployment_containment_relation(
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    source: &SymbolHandle,
    destination: &SymbolHandle,
) -> Option<DeploymentContainmentRelation> {
    if deployment_is_ancestor(documents_by_id, source, destination) {
        Some(DeploymentContainmentRelation::SourceAncestor)
    } else if deployment_is_ancestor(documents_by_id, destination, source) {
        Some(DeploymentContainmentRelation::DestinationAncestor)
    } else {
        None
    }
}

fn deployment_is_ancestor(
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    ancestor: &SymbolHandle,
    descendant: &SymbolHandle,
) -> bool {
    let mut current = deployment_parent_handle(documents_by_id, descendant);

    while let Some(parent_handle) = current {
        if &parent_handle == ancestor {
            return true;
        }
        current = deployment_parent_handle(documents_by_id, &parent_handle);
    }

    false
}

fn deployment_parent_handle(
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    handle: &SymbolHandle,
) -> Option<SymbolHandle> {
    let symbol = symbol_for_handle(documents_by_id, handle)?;
    let parent_id = symbol.parent?;
    let document = documents_by_id.get(handle.document())?;
    let parent = document.symbols.get(parent_id.0)?;
    if !is_deployment_element_kind(parent.kind) {
        return None;
    }

    Some(SymbolHandle::new(handle.document().clone(), parent.id))
}

fn response_relationship_is_in_view(
    source: &SymbolHandle,
    destination: &SymbolHandle,
    technology: Option<&str>,
    declared_relationships: &[DeclaredRelationship],
    seen_relationships: &BTreeSet<RelationshipLocation>,
) -> bool {
    declared_relationships.iter().any(|relationship| {
        relationship.source == *destination
            && relationship.destination == *source
            && relationship_technology_matches(relationship.technology.as_deref(), technology)
            && seen_relationships.contains(&RelationshipLocation::from_relationship(relationship))
    })
}

fn dynamic_relationship_annotation(
    primary_document: &DocumentId,
    source: &SymbolHandle,
    destination: &SymbolHandle,
    technology: Option<&str>,
    declared_relationships: &[DeclaredRelationship],
) -> Option<Annotation> {
    technology?;
    let candidate = declared_relationships.iter().find(|relationship| {
        relationship.source == *source && relationship.destination == *destination
    })?;
    let message = candidate.technology.as_deref().map_or_else(
        || "declared relationship here does not declare a technology".to_owned(),
        |existing| format!("declared relationship here uses technology {existing}"),
    );

    Some(secondary_annotation(
        primary_document,
        &candidate.document,
        candidate.span,
        message,
    ))
}

fn relationship_technology_matches(
    declared_technology: Option<&str>,
    expected_technology: Option<&str>,
) -> bool {
    expected_technology
        .is_none_or(|expected_technology| declared_technology == Some(expected_technology))
}

fn secondary_annotation(
    primary_document: &DocumentId,
    related_document: &DocumentId,
    span: TextSpan,
    message: impl Into<String>,
) -> Annotation {
    let annotation = if primary_document == related_document {
        Annotation::secondary(span)
    } else {
        Annotation::secondary(span).in_document(related_document)
    };
    annotation.message(message)
}

#[cfg(test)]
mod workspace_reference_tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn workspace_resolves_dotted_model_references_and_selector_targets() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                model {
                    !identifiers hierarchical

                    system = softwareSystem "System" {
                        api = container "API" {
                            worker = component "Worker"
                        }
                    }

                    !element system.api.worker {
                        properties {
                            "team" "Core"
                        }
                    }

                    system.api -> system.api.worker "Uses"
                }
            }
        "#});

        let workspace = WorkspaceLoader::new()
            .load_paths([fixture.workspace_path().as_path()])
            .expect("workspace should load");
        let index = workspace
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        let document_id = document_id_from_path(fixture.workspace_path());
        let document = workspace
            .document(&document_id)
            .expect("workspace document should exist");
        let snapshot = document.snapshot();

        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::ElementSelectorTarget,
            "system",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "system"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::ElementSelectorTarget,
            "system.api",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "api"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::ElementSelectorTarget,
            "system.api.worker",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "worker"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::RelationshipSource,
            "system.api",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "api"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::RelationshipDestination,
            "system.api.worker",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "worker"),
            ),
        );
    }

    #[test]
    fn workspace_resolves_dotted_deployment_references_and_selector_targets() {
        let fixture = TemporaryWorkspace::new(indoc! {r#"
            workspace {
                !identifiers hierarchical

                model {
                    system = softwareSystem "System" {
                        api = container "API"
                    }

                    live = deploymentEnvironment "Live" {
                        edge = deploymentNode "Edge" {
                            gateway = infrastructureNode "Gateway"
                            apiInstance = containerInstance system.api
                            live.edge.gateway -> live.edge.apiInstance "Routes"
                        }

                        !element live.edge.apiInstance {
                            properties {
                                "team" "Runtime"
                            }
                        }
                    }
                }
            }
        "#});

        let workspace = WorkspaceLoader::new()
            .load_paths([fixture.workspace_path().as_path()])
            .expect("workspace should load");
        let index = workspace
            .workspace_indexes()
            .first()
            .expect("workspace index should exist");
        let document_id = document_id_from_path(fixture.workspace_path());
        let document = workspace
            .document(&document_id)
            .expect("workspace document should exist");
        let snapshot = document.snapshot();

        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::ElementSelectorTarget,
            "live",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "live"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::ElementSelectorTarget,
            "live.edge",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "edge"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::ElementSelectorTarget,
            "live.edge.apiInstance",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "apiInstance"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::DeploymentRelationshipSource,
            "live.edge.gateway",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "gateway"),
            ),
        );
        assert_reference_resolves_to(
            index,
            &document_id,
            snapshot.references(),
            ReferenceKind::DeploymentRelationshipDestination,
            "live.edge.apiInstance",
            SymbolHandle::new(
                document_id.clone(),
                symbol_id_by_binding(snapshot.symbols(), "apiInstance"),
            ),
        );
    }
}
