// Deployment and filesystem-backed semantic diagnostics that validate resource
// paths and deployment topology after reference resolution succeeds.

fn deployment_semantic_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    // Deployment topology validation also works from resolved endpoint references,
    // but it stays separate from the view family because it reasons about
    // deployment containment rather than view composition. Keep the wrapper even
    // with one rule so later deployment-only checks have one obvious entry point.
    deployment_parent_child_relationship_diagnostics(
        documents,
        documents_by_id,
        bindings,
        reference_tables,
    )
}

fn deployment_parent_child_relationship_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
    bindings: &WorkspaceBindingTables,
    reference_tables: &WorkspaceReferenceTables,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for relationship in &document.relationship_facts {
            let Some(source_handle) = resolved_declared_relationship_endpoint(
                document,
                relationship.source.as_ref(),
                relationship.span,
                ReferenceKind::DeploymentRelationshipSource,
                ReferenceKind::RelationshipSource,
                bindings,
                reference_tables,
            ) else {
                continue;
            };
            let Some(destination_handle) = resolved_declared_relationship_endpoint(
                document,
                Some(&relationship.destination),
                relationship.span,
                ReferenceKind::DeploymentRelationshipDestination,
                ReferenceKind::RelationshipDestination,
                bindings,
                reference_tables,
            ) else {
                continue;
            };
            let Some(source_symbol) = symbol_for_handle(documents_by_id, &source_handle) else {
                continue;
            };
            let Some(destination_symbol) = symbol_for_handle(documents_by_id, &destination_handle)
            else {
                continue;
            };
            // Deployment endpoint references should already resolve to deployment
            // symbols. Keep the explicit guard so any broader future resolution
            // change fails closed instead of emitting topology diagnostics against
            // model-layer elements.
            if !is_deployment_element_kind(source_symbol.kind)
                || !is_deployment_element_kind(destination_symbol.kind)
            {
                continue;
            }
            let Some(relation) = deployment_containment_relation(
                documents_by_id,
                &source_handle,
                &destination_handle,
            ) else {
                continue;
            };

            let mut diagnostic = RuledDiagnostic::deployment_parent_child_relationship(
                &document.document_id,
                relationship.span,
            );
            let (ancestor_handle, ancestor_symbol, descendant_handle, descendant_symbol) =
                match relation {
                    DeploymentContainmentRelation::SourceAncestor => (
                        &source_handle,
                        source_symbol,
                        &destination_handle,
                        destination_symbol,
                    ),
                    DeploymentContainmentRelation::DestinationAncestor => (
                        &destination_handle,
                        destination_symbol,
                        &source_handle,
                        source_symbol,
                    ),
                };
            diagnostic.annotate(secondary_annotation(
                &document.document_id,
                ancestor_handle.document(),
                ancestor_symbol.span,
                format!(
                    "ancestor deployment element {} is declared here",
                    ancestor_symbol.display_name
                ),
            ));
            diagnostic.annotate(secondary_annotation(
                &document.document_id,
                descendant_handle.document(),
                descendant_symbol.span,
                format!(
                    "descendant deployment element {} is declared here",
                    descendant_symbol.display_name
                ),
            ));
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImageRendererRequirement {
    property_name: &'static str,
    service_name: &'static str,
}

fn resource_semantic_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    // Resource-path and image-view diagnostics form one filesystem-backed rule
    // family, so keep them together instead of interleaving them with view
    // topology checks.
    let mut diagnostics = documentation_resource_diagnostics(documents);
    diagnostics.extend(image_resource_diagnostics(documents));
    diagnostics
}

fn documentation_resource_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for directive in &document.resource_directives {
            let Some(path) =
                resolve_local_resource_path(document.document_location.as_ref(), &directive.path)
            else {
                continue;
            };
            let Some(message) = documentation_resource_path_message(directive, &path) else {
                continue;
            };
            diagnostics.push(RuledDiagnostic::invalid_documentation_path(
                &document.document_id,
                message,
                directive.path.span,
            ));
        }
    }

    diagnostics
}

fn documentation_resource_path_message(
    directive: &ResourceDirectiveFact,
    path: &Path,
) -> Option<String> {
    match fs::metadata(path) {
        Ok(metadata) => match directive.kind {
            // Upstream accepts any existing `!docs` path and lets the importer
            // decide whether that path is a directory or a single file.
            ResourceDirectiveKind::Docs => None,
            ResourceDirectiveKind::Adrs if metadata.is_dir() => None,
            ResourceDirectiveKind::Adrs => Some(format!(
                "Documentation path {} is not a directory",
                path.display()
            )),
        },
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Some(format!(
                "Documentation path {} does not exist",
                path.display()
            ))
        }
        Err(error) => Some(format!(
            "Error inspecting documentation path {}: {}",
            path.display(),
            error
        )),
    }
}

fn image_resource_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let viewset_property_names = documents
        .iter()
        .filter(|document| !document.has_syntax_errors)
        .flat_map(|document| {
            document
                .property_facts
                .iter()
                .filter(|property| property.container_node_kind == "views_block")
                .map(|property| property.name.normalized_text.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for view in &document.view_facts {
            if view.kind != ViewKind::Image {
                continue;
            }

            for source in &view.image_sources {
                if let Some(requirement) = required_image_renderer(source.kind)
                    && !image_renderer_property_is_defined(
                        document,
                        view,
                        &viewset_property_names,
                        requirement.property_name,
                    )
                {
                    diagnostics.push(RuledDiagnostic::missing_image_renderer_property(
                        &document.document_id,
                        requirement.property_name,
                        requirement.service_name,
                        source.span,
                    ));
                }

                let Some(path) =
                    resolve_local_resource_path(document.document_location.as_ref(), &source.value)
                else {
                    continue;
                };
                let Some(message) = image_source_path_message(source.kind, &path) else {
                    continue;
                };
                diagnostics.push(RuledDiagnostic::invalid_image_source(
                    &document.document_id,
                    message,
                    source.value.span,
                ));
            }
        }
    }

    diagnostics
}

const fn required_image_renderer(kind: ImageSourceKind) -> Option<ImageRendererRequirement> {
    Some(match kind {
        ImageSourceKind::PlantUml => ImageRendererRequirement {
            property_name: "plantuml.url",
            service_name: "PlantUML",
        },
        ImageSourceKind::Mermaid => ImageRendererRequirement {
            property_name: "mermaid.url",
            service_name: "Mermaid",
        },
        ImageSourceKind::Kroki => ImageRendererRequirement {
            property_name: "kroki.url",
            service_name: "Kroki",
        },
        ImageSourceKind::Image => return None,
    })
}

fn image_renderer_property_is_defined(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    viewset_property_names: &BTreeSet<String>,
    property_name: &str,
) -> bool {
    viewset_property_names.contains(property_name)
        || image_view_local_property_is_defined(document, view, property_name)
}

fn image_view_local_property_is_defined(
    document: &WorkspaceSemanticDocumentFacts,
    view: &ViewFact,
    property_name: &str,
) -> bool {
    let Some(body_span) = view.body_span else {
        return false;
    };

    document.property_facts.iter().any(|property| {
        property.name.normalized_text == property_name && span_within(body_span, property.span)
    })
}

fn image_source_path_message(kind: ImageSourceKind, path: &Path) -> Option<String> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => None,
        Ok(metadata) if metadata.is_dir() => Some(match kind {
            ImageSourceKind::Image => format!("{} is not a file", path.display()),
            ImageSourceKind::PlantUml | ImageSourceKind::Mermaid | ImageSourceKind::Kroki => {
                // The upstream importers attempt to read these sources as files
                // and therefore surface the platform's terse "Is a directory"
                // failure. Preserve that shape for parity.
                "Is a directory".to_owned()
            }
        }),
        Ok(_) => Some(format!("{} is not a file", path.display())),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Some(format!("The file at {} does not exist", path.display()))
        }
        Err(error) => Some(format!(
            "Error inspecting image source {}: {}",
            path.display(),
            error
        )),
    }
}
