// Generic structure and binding diagnostics that apply across workspace
// instances before the more specialized deployment, resource, and view rules.

fn emits_generic_reference_diagnostics(reference: &Reference) -> bool {
    // Dotted `!element` targets contribute one reference per segment for
    // navigation (`system`, `system.api`, ...), but selector diagnostics still
    // belong to the directive facts. Keep that policy in one helper so future
    // consumers do not need to rediscover why selector references are special.
    reference.kind != ReferenceKind::ElementSelectorTarget
}

fn unresolved_reference_diagnostic(
    document: &DocumentId,
    reference: &Reference,
) -> RuledDiagnostic {
    RuledDiagnostic::unresolved_reference(
        document,
        reference_target_hint_label(reference.target_hint),
        &reference.raw_text,
        reference.span,
    )
}

fn ambiguous_reference_diagnostic(
    document: &DocumentId,
    reference: &Reference,
    status: &ReferenceResolutionStatus,
) -> RuledDiagnostic {
    RuledDiagnostic::ambiguous_reference(
        document,
        Some(reference_target_hint_label(reference.target_hint)),
        &reference.raw_text,
        Some(ambiguous_reference_reason(status)),
        reference.span,
    )
}

const fn reference_target_hint_label(target_hint: ReferenceTargetHint) -> &'static str {
    match target_hint {
        ReferenceTargetHint::Element => "element",
        ReferenceTargetHint::ElementOrDeployment => "element or deployment",
        ReferenceTargetHint::Deployment => "deployment",
        ReferenceTargetHint::Relationship => "relationship",
        ReferenceTargetHint::ElementOrRelationship => "element or relationship",
    }
}

const fn ambiguous_reference_reason(status: &ReferenceResolutionStatus) -> &'static str {
    match status {
        ReferenceResolutionStatus::AmbiguousDuplicateBinding => "multiple bindings match",
        ReferenceResolutionStatus::AmbiguousElementVsRelationship => {
            "both an element binding and a relationship binding match"
        }
        ReferenceResolutionStatus::Resolved(_)
        | ReferenceResolutionStatus::UnresolvedNoMatch
        | ReferenceResolutionStatus::DeferredByScopePolicy => "resolution is not unique",
    }
}

fn split_binding_table(
    bindings: BTreeMap<String, Vec<SymbolHandle>>,
) -> (
    BTreeMap<String, SymbolHandle>,
    BTreeMap<String, Vec<SymbolHandle>>,
) {
    let mut unique = BTreeMap::new();
    let mut duplicates = BTreeMap::new();

    for (key, mut handles) in bindings {
        handles.sort();
        handles.dedup();

        if let [handle] = handles.as_slice() {
            unique.insert(key, handle.clone());
        } else {
            duplicates.insert(key, handles);
        }
    }

    (unique, duplicates)
}

fn push_duplicate_binding_diagnostics(
    binding_kind: &str,
    duplicate_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
    documents: &[&WorkspaceSemanticDocumentFacts],
    diagnostics: &mut Vec<RuledDiagnostic>,
) {
    let documents_by_id = documents
        .iter()
        .map(|document| (&document.document_id, *document))
        .collect::<BTreeMap<_, _>>();

    for (key, handles) in duplicate_bindings {
        let duplicate_sites = handles
            .iter()
            .filter_map(|handle| {
                let document = documents_by_id
                    .get(handle.document())
                    .expect("BUG: duplicate-binding document should exist");
                if document.has_syntax_errors {
                    return None;
                }

                let symbol = document
                    .symbols
                    .get(handle.symbol_id().0)
                    .expect("BUG: duplicate-binding symbol should exist");
                Some((handle, symbol.span))
            })
            .collect::<Vec<_>>();

        for (handle, span) in &duplicate_sites {
            let mut diagnostic =
                RuledDiagnostic::duplicate_binding(handle.document(), binding_kind, key, *span);
            for (related_handle, related_span) in &duplicate_sites {
                if related_handle == handle {
                    continue;
                }
                diagnostic.annotate(secondary_annotation(
                    handle.document(),
                    related_handle.document(),
                    *related_span,
                    format!("other {binding_kind} binding for {key} is declared here"),
                ));
            }
            diagnostics.push(diagnostic);
        }
    }
}

fn workspace_structure_diagnostics(
    definition_documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();
    push_repeated_workspace_section_diagnostics(
        WorkspaceSectionKind::Model,
        "model",
        definition_documents,
        &mut diagnostics,
    );
    push_repeated_workspace_section_diagnostics(
        WorkspaceSectionKind::Views,
        "views",
        definition_documents,
        &mut diagnostics,
    );
    diagnostics
}

fn push_repeated_workspace_section_diagnostics(
    section_kind: WorkspaceSectionKind,
    section_name: &str,
    documents: &[&WorkspaceSemanticDocumentFacts],
    diagnostics: &mut Vec<RuledDiagnostic>,
) {
    let occurrences = documents
        .iter()
        .filter(|document| !document.has_syntax_errors)
        .flat_map(|document| {
            document
                .workspace_sections
                .iter()
                .filter(move |fact| fact.kind == section_kind)
                .map(move |fact| (document.document_id.clone(), fact.span))
        })
        .collect::<Vec<_>>();
    let Some((first_document, first_span)) = occurrences.first().cloned() else {
        return;
    };

    for (document, span) in occurrences.into_iter().skip(1) {
        let mut diagnostic =
            RuledDiagnostic::repeated_workspace_section(&document, section_name, span);
        let annotation = if document == first_document {
            Annotation::secondary(first_span)
        } else {
            Annotation::secondary(first_span).in_document(&first_document)
        }
        .message(format!("first {section_name} section here"));
        diagnostic.annotate(annotation);
        diagnostics.push(diagnostic);
    }
}

fn workspace_scope_diagnostics(
    definition_documents: &[&WorkspaceSemanticDocumentFacts],
    instance_documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<RuledDiagnostic> {
    let Some((scope_document, scope_fact)) = effective_workspace_scope(definition_documents)
        .or_else(|| effective_workspace_scope(instance_documents))
    else {
        return Vec::new();
    };
    let violations = workspace_scope_violations(&scope_fact.scope, instance_documents);
    if violations.is_empty() {
        return Vec::new();
    }

    violations
        .into_iter()
        .map(|violation| {
            let message = format!(
                "workspace is {} scoped, but the {} named {} has {}",
                workspace_scope_label(&scope_fact.scope),
                scope_violation_owner_label(&violation.owner),
                violation.owner.display_name,
                violation.child_plural,
            );
            let mut diagnostic = RuledDiagnostic::workspace_scope_mismatch(
                &scope_document,
                message,
                scope_fact.span,
            );
            let annotation = if scope_document == violation.document {
                Annotation::secondary(violation.owner.span)
            } else {
                Annotation::secondary(violation.owner.span).in_document(&violation.document)
            }
            .message(format!(
                "{} named {} has {}",
                scope_violation_owner_label(&violation.owner),
                violation.owner.display_name,
                violation.child_plural,
            ));
            diagnostic.annotate(annotation);
            diagnostic
        })
        .collect()
}

fn effective_workspace_scope(
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Option<(DocumentId, ConfigurationScopeFact)> {
    // Definition documents arrive in root-first include order. Prefer the first
    // scope we encounter so an included fragment cannot silently override the
    // root workspace entry's explicit scope declaration.
    documents.iter().find_map(|document| {
        document
            .configuration_scopes
            .last()
            .cloned()
            .map(|fact| (document.document_id.clone(), fact))
    })
}

#[derive(Debug)]
struct WorkspaceScopeViolation {
    document: DocumentId,
    owner: Symbol,
    child_plural: &'static str,
}

fn workspace_scope_violations(
    scope: &WorkspaceScope,
    documents: &[&WorkspaceSemanticDocumentFacts],
) -> Vec<WorkspaceScopeViolation> {
    match scope {
        WorkspaceScope::Landscape => {
            scope_violations_for_child_kind(documents, SymbolKind::Container, "containers")
        }
        WorkspaceScope::SoftwareSystem => {
            scope_violations_for_child_kind(documents, SymbolKind::Component, "components")
        }
        WorkspaceScope::Container | WorkspaceScope::Component | WorkspaceScope::Other(_) => {
            Vec::new()
        }
    }
}

fn scope_violations_for_child_kind(
    documents: &[&WorkspaceSemanticDocumentFacts],
    child_kind: SymbolKind,
    child_plural: &'static str,
) -> Vec<WorkspaceScopeViolation> {
    let mut seen_owners = BTreeSet::new();
    let mut violations = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for symbol in &document.symbols {
            if symbol.kind != child_kind {
                continue;
            }

            let Some(owner) = scope_violation_owner(&document.symbols, symbol.parent).cloned()
            else {
                continue;
            };
            if seen_owners.insert((document.document_id.clone(), owner.id)) {
                violations.push(WorkspaceScopeViolation {
                    document: document.document_id.clone(),
                    owner,
                    child_plural,
                });
            }
        }
    }

    violations
}

fn scope_violation_owner(symbols: &[Symbol], mut parent: Option<SymbolId>) -> Option<&Symbol> {
    while let Some(parent_id) = parent {
        let owner = symbols.get(parent_id.0)?;
        match owner.kind {
            SymbolKind::SoftwareSystem | SymbolKind::Container => return Some(owner),
            _ => parent = owner.parent,
        }
    }

    None
}

const fn scope_violation_owner_label(symbol: &Symbol) -> &'static str {
    match symbol.kind {
        SymbolKind::SoftwareSystem => "software system",
        SymbolKind::Container => "container",
        _ => "element",
    }
}

const fn workspace_scope_label(scope: &WorkspaceScope) -> &str {
    match scope {
        WorkspaceScope::Landscape => "landscape",
        WorkspaceScope::SoftwareSystem => "software system",
        WorkspaceScope::Container => "container",
        WorkspaceScope::Component => "component",
        WorkspaceScope::Other(raw) => raw.as_str(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ViewLocation {
    document: DocumentId,
    view_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RelationshipLocation {
    document: DocumentId,
    span: TextSpan,
}

impl RelationshipLocation {
    fn from_relationship(relationship: &DeclaredRelationship) -> Self {
        Self {
            document: relationship.document.clone(),
            span: relationship.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredRelationship {
    handle: Option<SymbolHandle>,
    document: DocumentId,
    span: TextSpan,
    source: SymbolHandle,
    destination: SymbolHandle,
    technology: Option<String>,
}

// The dynamic-view rules share one interpretation phase before they diverge into
// separate diagnostics:
//
// 1. resolve scope once
// 2. resolve each step into concrete handles or declared relationships
// 3. apply per-rule policy such as scope redundancy or request/response ordering
//
// Keeping that intermediate form explicit makes the order-sensitive response
// logic easier to read and avoids duplicating the same reference lookups in both
// dynamic-view passes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedDynamicView {
    scope: Option<ResolvedDynamicScope>,
    steps: Vec<ResolvedDynamicStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedDynamicScope {
    span: TextSpan,
    handle: SymbolHandle,
    display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedDynamicStep {
    Relationship {
        span: TextSpan,
        source: SymbolHandle,
        destination: SymbolHandle,
        source_name: String,
        destination_name: String,
        technology: Option<String>,
    },
    RelationshipReference {
        span: TextSpan,
        relationship: DeclaredRelationship,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeploymentContainmentRelation {
    SourceAncestor,
    DestinationAncestor,
}
