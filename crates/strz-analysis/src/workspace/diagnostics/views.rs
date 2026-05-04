// View-specific semantic diagnostics, including element-family checks and the
// dynamic-view interpretation phase shared by multiple rules.

fn view_semantic_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    // All four view rules read from the same extracted `ViewFact` surface, so keep
    // them together as one post-reference pass instead of scattering view-specific
    // checks across unrelated binding code paths.
    let views_by_key = index_views_by_key(documents);
    let declared_relationships =
        collect_declared_relationships(documents, documents_by_id, bindings, reference_tables);

    let mut diagnostics = Vec::new();
    diagnostics.extend(filtered_view_autolayout_diagnostics(
        documents,
        documents_by_id,
        &views_by_key,
    ));
    diagnostics.extend(invalid_view_element_diagnostics(
        documents,
        documents_by_id,
        reference_tables,
    ));
    diagnostics.extend(dynamic_view_scope_redundancy_diagnostics(
        documents,
        documents_by_id,
        reference_tables,
        &declared_relationships,
    ));
    diagnostics.extend(dynamic_view_relationship_diagnostics(
        documents,
        documents_by_id,
        reference_tables,
        &declared_relationships,
    ));
    diagnostics
}

fn filtered_view_autolayout_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    views_by_key: &BTreeMap<String, Vec<ViewLocation>>,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            if view.kind != ViewKind::Filtered {
                continue;
            }

            let Some(base_key) = view.base_key.as_ref() else {
                continue;
            };
            let Some(base_view_locations) = views_by_key.get(&base_key.normalized_text) else {
                continue;
            };
            let [base_view_location] = base_view_locations.as_slice() else {
                continue;
            };
            let Some(base_document) = documents_by_id.get(&base_view_location.document) else {
                continue;
            };
            if base_document.has_syntax_errors {
                continue;
            }
            let Some(base_view) = base_document.view_facts.get(base_view_location.view_index)
            else {
                continue;
            };
            let Some(auto_layout) = base_view.auto_layout.as_ref() else {
                continue;
            };

            let mut diagnostic = RuledDiagnostic::filtered_view_autolayout_mismatch(
                &document.document_id,
                &base_key.normalized_text,
                base_key.span,
            );
            diagnostic.annotate(secondary_annotation(
                &document.document_id,
                &base_document.document_id,
                auto_layout.span,
                "base view enables automatic layout here",
            ));
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

fn invalid_view_element_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            if !matches!(
                view.kind,
                ViewKind::SystemLandscape
                    | ViewKind::SystemContext
                    | ViewKind::Container
                    | ViewKind::Component
            ) {
                continue;
            }

            push_invalid_view_value_diagnostics(
                document,
                view,
                &view.include_values,
                ReferenceKind::ViewInclude,
                documents_by_id,
                reference_tables,
                &mut diagnostics,
            );
            push_invalid_view_value_diagnostics(
                document,
                view,
                &view.animation_values,
                ReferenceKind::ViewAnimation,
                documents_by_id,
                reference_tables,
                &mut diagnostics,
            );
        }
    }

    diagnostics
}

fn push_invalid_view_value_diagnostics(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    values: &[ValueFact],
    reference_kind: ReferenceKind,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
    diagnostics: &mut Vec<RuledDiagnostic>,
) {
    for value in values {
        let Some(target_handle) =
            resolved_reference_target(document, reference_kind, value.span, reference_tables)
        else {
            continue;
        };
        let Some(target_symbol) = symbol_for_handle(documents_by_id, &target_handle) else {
            continue;
        };
        if target_symbol.kind == SymbolKind::Relationship {
            continue;
        }
        if is_view_element_kind_allowed(view.kind, target_symbol.kind) {
            continue;
        }

        diagnostics.push(RuledDiagnostic::invalid_view_element(
            &document.document_id,
            &value.normalized_text,
            value.span,
        ));
    }
}

fn dynamic_view_scope_redundancy_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            let Some(resolved_view) = resolved_dynamic_view(
                document,
                view,
                documents_by_id,
                reference_tables,
                declared_relationships,
            ) else {
                continue;
            };
            let Some(scope) = resolved_view.scope.as_ref() else {
                continue;
            };

            for step in &resolved_view.steps {
                match step {
                    ResolvedDynamicStep::Relationship {
                        span,
                        source,
                        destination,
                        ..
                    } => {
                        if *source != scope.handle && *destination != scope.handle {
                            continue;
                        }

                        let diagnostic =
                            dynamic_view_scope_diagnostic(&document.document_id, scope, *span);
                        diagnostics.push(diagnostic);
                    }
                    ResolvedDynamicStep::RelationshipReference { span, relationship } => {
                        if relationship.source != scope.handle
                            && relationship.destination != scope.handle
                        {
                            continue;
                        }

                        let mut diagnostic =
                            dynamic_view_scope_diagnostic(&document.document_id, scope, *span);
                        diagnostic.annotate(dynamic_view_scope_relationship_annotation(
                            &document.document_id,
                            &scope.display_name,
                            relationship,
                        ));
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    diagnostics
}

fn dynamic_view_scope_diagnostic(
    document: &DocumentId,
    scope: &ResolvedDynamicScope,
    step_span: TextSpan,
) -> RuledDiagnostic {
    let mut diagnostic =
        RuledDiagnostic::dynamic_view_scope_redundancy(document, &scope.display_name, step_span);
    diagnostic.annotate(secondary_annotation(
        document,
        document,
        scope.span,
        "view scope is declared here",
    ));
    diagnostic
}

fn dynamic_view_scope_relationship_annotation(
    primary_document: &DocumentId,
    scope_name: &str,
    relationship: &DeclaredRelationship,
) -> Annotation {
    secondary_annotation(
        primary_document,
        &relationship.document,
        relationship.span,
        format!("referenced relationship here already includes {scope_name}"),
    )
}

fn dynamic_view_relationship_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            let Some(resolved_view) = resolved_dynamic_view(
                document,
                view,
                documents_by_id,
                reference_tables,
                declared_relationships,
            ) else {
                continue;
            };

            // Upstream treats a reverse-direction step as a valid response only
            // after the forward request has already appeared in the same dynamic
            // view. Keep that history explicit so both identifier-written steps and
            // named-relationship steps participate in the same ordering rule.
            let scope_handle = resolved_view.scope.as_ref().map(|scope| &scope.handle);
            let mut seen_relationships = BTreeSet::<RelationshipLocation>::new();

            for step in &resolved_view.steps {
                match step {
                    ResolvedDynamicStep::RelationshipReference { relationship, .. } => {
                        seen_relationships
                            .insert(RelationshipLocation::from_relationship(relationship));
                    }
                    ResolvedDynamicStep::Relationship {
                        span,
                        source,
                        destination,
                        source_name,
                        destination_name,
                        technology,
                    } => {
                        if scope_handle.is_some_and(|scope_handle| {
                            *scope_handle == *source || *scope_handle == *destination
                        }) {
                            continue;
                        }
                        let technology = technology.as_deref();

                        if let Some(relationship) = matching_declared_relationship(
                            source,
                            destination,
                            technology,
                            declared_relationships,
                        ) {
                            seen_relationships
                                .insert(RelationshipLocation::from_relationship(relationship));
                            continue;
                        }

                        if response_relationship_is_in_view(
                            source,
                            destination,
                            technology,
                            declared_relationships,
                            &seen_relationships,
                        ) {
                            continue;
                        }

                        let mut diagnostic = RuledDiagnostic::dynamic_view_relationship_mismatch(
                            &document.document_id,
                            source_name,
                            destination_name,
                            technology,
                            *span,
                        );
                        if let Some(annotation) = dynamic_relationship_annotation(
                            &document.document_id,
                            source,
                            destination,
                            technology,
                            declared_relationships,
                        ) {
                            diagnostic.annotate(annotation);
                        }
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    diagnostics
}

fn resolved_dynamic_view(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Option<ResolvedDynamicView> {
    if view.kind != ViewKind::Dynamic {
        return None;
    }

    let scope = view.scope.as_ref().and_then(|scope| {
        let handle = resolved_reference_target(
            document,
            ReferenceKind::ViewScope,
            scope.span,
            reference_tables,
        )?;
        let symbol = symbol_for_handle(documents_by_id, &handle)?;
        Some(ResolvedDynamicScope {
            span: scope.span,
            handle,
            display_name: symbol.display_name.clone(),
        })
    });

    let steps = view
        .dynamic_steps
        .iter()
        .filter_map(|step| {
            // If a step cannot be resolved, the reference layer has already
            // emitted the appropriate unresolved/ambiguous diagnostic. The
            // view-specific passes only need the successfully resolved subset.
            resolve_dynamic_step(
                document,
                step,
                documents_by_id,
                reference_tables,
                declared_relationships,
            )
        })
        .collect();

    Some(ResolvedDynamicView { scope, steps })
}

fn resolve_dynamic_step(
    document: &WorkspaceSemanticDocumentFacts,
    step: &DynamicViewStepFact,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    reference_tables: &WorkspaceReferenceTables,
    declared_relationships: &[DeclaredRelationship],
) -> Option<ResolvedDynamicStep> {
    match step {
        DynamicViewStepFact::Relationship(step) => {
            let source = resolved_reference_target(
                document,
                ReferenceKind::RelationshipSource,
                step.source.span,
                reference_tables,
            )?;
            let destination = resolved_reference_target(
                document,
                ReferenceKind::RelationshipDestination,
                step.destination.span,
                reference_tables,
            )?;
            let source_symbol = symbol_for_handle(documents_by_id, &source)?;
            let destination_symbol = symbol_for_handle(documents_by_id, &destination)?;

            Some(ResolvedDynamicStep::Relationship {
                span: step.span,
                source,
                destination,
                source_name: source_symbol.display_name.clone(),
                destination_name: destination_symbol.display_name.clone(),
                technology: step
                    .technology
                    .as_ref()
                    .map(|value| value.normalized_text.clone()),
            })
        }
        DynamicViewStepFact::RelationshipReference(step) => {
            let relationship_handle = resolved_reference_target(
                document,
                ReferenceKind::DynamicRelationshipReference,
                step.relationship.span,
                reference_tables,
            )?;
            let relationship =
                declared_relationship_for_handle(&relationship_handle, declared_relationships)?;

            Some(ResolvedDynamicStep::RelationshipReference {
                span: step.span,
                relationship: relationship.clone(),
            })
        }
    }
}

fn index_views_by_key(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> BTreeMap<String, Vec<ViewLocation>> {
    let mut views_by_key = BTreeMap::<String, Vec<ViewLocation>>::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for (view_index, view) in document.view_facts.iter().enumerate() {
            let Some(key) = view.key.as_ref() else {
                continue;
            };
            views_by_key
                .entry(key.normalized_text.clone())
                .or_default()
                .push(ViewLocation {
                    document: document.document_id.clone(),
                    view_index,
                });
        }
    }

    views_by_key
}

#[cfg(test)]
mod workspace_view_tests {
    use super::*;

    #[test]
    fn view_element_matrix_matches_current_upstream_parity() {
        assert!(is_view_element_kind_allowed(
            ViewKind::SystemLandscape,
            SymbolKind::SoftwareSystem
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::SystemLandscape,
            SymbolKind::Container
        ));

        assert!(is_view_element_kind_allowed(
            ViewKind::SystemContext,
            SymbolKind::Person
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::SystemContext,
            SymbolKind::Container
        ));

        assert!(is_view_element_kind_allowed(
            ViewKind::Container,
            SymbolKind::Container
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::Container,
            SymbolKind::Component
        ));

        assert!(is_view_element_kind_allowed(
            ViewKind::Component,
            SymbolKind::Component
        ));
        assert!(!is_view_element_kind_allowed(
            ViewKind::Component,
            SymbolKind::Relationship
        ));
    }
}
