// Element-selector resolution and selector-specific diagnostics layered on top
// of the shared reference and binding infrastructure.

fn element_selector_diagnostics(
    documents: &[&WorkspaceSemanticDocumentFacts],
    bindings: &WorkspaceBindingTables,
) -> Vec<RuledDiagnostic> {
    let mut diagnostics = Vec::new();

    for document in documents {
        if document.has_syntax_errors {
            continue;
        }

        for directive in &document.element_directives {
            // TODO: Path-style selectors such as `DeploymentNode://...` need a
            // richer resolver than the current binding tables.
            if !matches!(
                directive.target.value_kind,
                DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
            ) {
                continue;
            }

            match resolve_element_selector_target(document, directive, bindings) {
                SelectorResolutionStatus::Resolved => {}
                SelectorResolutionStatus::UnresolvedNoMatch => {
                    diagnostics.push(RuledDiagnostic::unresolved_element_selector(
                        &document.document_id,
                        &directive.target.normalized_text,
                        directive.target.span,
                    ));
                }
                SelectorResolutionStatus::Ambiguous => {
                    diagnostics.push(RuledDiagnostic::ambiguous_reference(
                        &document.document_id,
                        None,
                        &directive.target.normalized_text,
                        None,
                        directive.target.span,
                    ));
                }
            }
        }
    }

    diagnostics
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectorResolutionStatus {
    Resolved,
    UnresolvedNoMatch,
    Ambiguous,
}

enum SelectorTargetHandleResolution {
    Resolved(SymbolHandle),
    UnresolvedNoMatch,
    Ambiguous,
}

fn resolve_element_selector_target(
    document: &WorkspaceSemanticDocumentFacts,
    directive: &ElementDirectiveFact,
    bindings: &WorkspaceBindingTables,
) -> SelectorResolutionStatus {
    match resolve_element_selector_target_handle(document, directive, bindings) {
        SelectorTargetHandleResolution::Resolved(_) => SelectorResolutionStatus::Resolved,
        SelectorTargetHandleResolution::UnresolvedNoMatch => {
            SelectorResolutionStatus::UnresolvedNoMatch
        }
        SelectorTargetHandleResolution::Ambiguous => SelectorResolutionStatus::Ambiguous,
    }
}

fn resolve_element_selector_target_handle(
    document: &WorkspaceSemanticDocumentFacts,
    directive: &ElementDirectiveFact,
    bindings: &WorkspaceBindingTables,
) -> SelectorTargetHandleResolution {
    for candidate in element_selector_target_candidates(document, directive, bindings) {
        let contextual_status = resolve_selector_target_handle_raw_text(&candidate, bindings);
        if !matches!(
            contextual_status,
            SelectorTargetHandleResolution::UnresolvedNoMatch
        ) {
            return contextual_status;
        }
    }

    SelectorTargetHandleResolution::UnresolvedNoMatch
}

fn resolve_selector_target_raw_text(
    raw_text: &str,
    bindings: &WorkspaceBindingTables,
) -> SelectorResolutionStatus {
    if bindings.duplicate_elements.contains_key(raw_text)
        || bindings.duplicate_deployments.contains_key(raw_text)
    {
        return SelectorResolutionStatus::Ambiguous;
    }

    match (
        bindings.unique_elements.get(raw_text),
        bindings.unique_deployments.get(raw_text),
    ) {
        (Some(_), Some(_)) => SelectorResolutionStatus::Ambiguous,
        (Some(_), None) | (None, Some(_)) => SelectorResolutionStatus::Resolved,
        (None, None) => SelectorResolutionStatus::UnresolvedNoMatch,
    }
}

fn resolve_selector_target_handle_raw_text(
    raw_text: &str,
    bindings: &WorkspaceBindingTables,
) -> SelectorTargetHandleResolution {
    if bindings.duplicate_elements.contains_key(raw_text)
        || bindings.duplicate_deployments.contains_key(raw_text)
    {
        return SelectorTargetHandleResolution::Ambiguous;
    }

    match (
        bindings.unique_elements.get(raw_text),
        bindings.unique_deployments.get(raw_text),
    ) {
        (Some(_), Some(_)) => SelectorTargetHandleResolution::Ambiguous,
        (Some(handle), None) | (None, Some(handle)) => {
            SelectorTargetHandleResolution::Resolved(handle.clone())
        }
        (None, None) => SelectorTargetHandleResolution::UnresolvedNoMatch,
    }
}

fn enclosing_symbol_for_span(
    document: &WorkspaceSemanticDocumentFacts,
    span: TextSpan,
) -> Option<SymbolId> {
    document
        .symbols
        .iter()
        .filter(|symbol| span_within(symbol.span, span))
        .min_by_key(|symbol| symbol.span.end_byte - symbol.span.start_byte)
        .map(|symbol| symbol.id)
}

fn resolve_reference_status(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
) -> ReferenceResolutionStatus {
    if reference.kind == ReferenceKind::ElementSelectorTarget {
        return resolve_selector_segment_reference(document, reference, bindings, documents_by_id);
    }

    if is_contextual_this_reference(reference) {
        return resolve_this_reference_status(document, reference, bindings);
    }

    // Syntax-role kinds remain useful to the LSP and diagnostics, but bounded
    // workspace resolution really depends on which binding family one reference
    // is allowed to target.
    let status = match reference.kind {
        ReferenceKind::RelationshipSource
        | ReferenceKind::RelationshipDestination
        | ReferenceKind::DynamicRelationshipReference
        | ReferenceKind::InstanceTarget
        | ReferenceKind::ElementSelectorTarget
        | ReferenceKind::DeploymentRelationshipSource
        | ReferenceKind::DeploymentRelationshipDestination
        | ReferenceKind::ViewScope
        | ReferenceKind::ViewInclude
        | ReferenceKind::ViewExclude
        | ReferenceKind::ViewAnimation => {
            resolve_reference_against_target_hint(reference, bindings)
        }
    };

    if status == ReferenceResolutionStatus::UnresolvedNoMatch {
        let contextual_status =
            resolve_reference_with_symbol_context(document, reference, bindings);
        if contextual_status == ReferenceResolutionStatus::UnresolvedNoMatch {
            resolve_reference_with_selector_context(document, reference, bindings)
        } else {
            contextual_status
        }
    } else {
        status
    }
}

fn resolve_selector_segment_reference(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
) -> ReferenceResolutionStatus {
    let Some(directive) = enclosing_element_directive(document, reference.span) else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };
    if !matches!(
        directive.target.value_kind,
        DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
    ) {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    }

    let resolved_target =
        match resolve_element_selector_target_handle(document, directive, bindings) {
            SelectorTargetHandleResolution::Resolved(handle) => handle,
            SelectorTargetHandleResolution::Ambiguous => {
                return ReferenceResolutionStatus::AmbiguousDuplicateBinding;
            }
            SelectorTargetHandleResolution::UnresolvedNoMatch => {
                return ReferenceResolutionStatus::UnresolvedNoMatch;
            }
        };
    if reference.span == directive.target.span || !directive.target.normalized_text.contains('.') {
        return ReferenceResolutionStatus::Resolved(resolved_target);
    }
    let Some(segment_index) = selector_segment_index(&directive.target, reference.span) else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };

    selector_segment_handle(&resolved_target, segment_index, documents_by_id).map_or(
        ReferenceResolutionStatus::UnresolvedNoMatch,
        ReferenceResolutionStatus::Resolved,
    )
}

fn is_contextual_this_reference(reference: &Reference) -> bool {
    reference.raw_text == "this"
        && matches!(
            reference.kind,
            ReferenceKind::RelationshipSource
                | ReferenceKind::RelationshipDestination
                | ReferenceKind::DeploymentRelationshipSource
                | ReferenceKind::DeploymentRelationshipDestination
        )
}

fn resolve_this_reference_status(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    let start_symbol = reference
        .containing_symbol
        .or_else(|| enclosing_symbol_for_span(document, reference.span));
    let Some(target) = contextual_owner_target(reference.target_hint) else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };

    match contextual_owner_resolution(document, reference.span, start_symbol, target, bindings) {
        ContextualOwnerResolution::Resolved(handle) => ReferenceResolutionStatus::Resolved(handle),
        ContextualOwnerResolution::Ambiguous => {
            ReferenceResolutionStatus::AmbiguousDuplicateBinding
        }
        ContextualOwnerResolution::Unresolved => ReferenceResolutionStatus::UnresolvedNoMatch,
    }
}

const fn contextual_owner_target(
    target_hint: ReferenceTargetHint,
) -> Option<ContextualOwnerTarget> {
    match target_hint {
        ReferenceTargetHint::Element => Some(ContextualOwnerTarget::Element),
        ReferenceTargetHint::ElementOrDeployment => {
            Some(ContextualOwnerTarget::ElementOrDeployment)
        }
        ReferenceTargetHint::Deployment => Some(ContextualOwnerTarget::Deployment),
        ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => None,
    }
}

fn contextual_symbol_target_handle_for_owner(
    document: &WorkspaceSemanticDocumentFacts,
    start_symbol: Option<SymbolId>,
    target: ContextualOwnerTarget,
) -> Option<SymbolHandle> {
    let matches_kind: fn(SymbolKind) -> bool = match target {
        ContextualOwnerTarget::Element => is_model_element_kind,
        ContextualOwnerTarget::Deployment => is_deployment_element_kind,
        ContextualOwnerTarget::ElementOrDeployment => {
            |kind| is_model_element_kind(kind) || is_deployment_element_kind(kind)
        }
    };

    contextual_symbol_target_handle_from_matcher(document, start_symbol, matches_kind)
}

fn contextual_symbol_target_handle_from_matcher(
    document: &WorkspaceSemanticDocumentFacts,
    start_symbol: Option<SymbolId>,
    matches_kind: fn(SymbolKind) -> bool,
) -> Option<SymbolHandle> {
    let mut current = start_symbol;
    while let Some(symbol_id) = current {
        let symbol = document
            .symbols
            .get(symbol_id.0)
            .expect("BUG: contextual symbol should exist");
        if matches_kind(symbol.kind) {
            return Some(SymbolHandle::new(document.document_id.clone(), symbol.id));
        }
        current = symbol.parent;
    }

    None
}

fn resolve_reference_with_symbol_context(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    let Some(containing_symbol) = reference.containing_symbol else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };
    let Some(mode) = bindings.element_modes.get(&document.document_id).copied() else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };

    for prefix in contextual_reference_prefixes(
        &document.symbols,
        containing_symbol,
        mode,
        reference.target_hint,
    ) {
        let contextual_raw_text = format!("{prefix}.{}", reference.raw_text);
        let status = match reference.target_hint {
            ReferenceTargetHint::Element => resolve_reference_against_element_table(
                &contextual_raw_text,
                &bindings.unique_elements,
                &bindings.duplicate_elements,
            ),
            ReferenceTargetHint::ElementOrDeployment => {
                resolve_reference_against_element_or_deployment_tables(
                    &contextual_raw_text,
                    &bindings.unique_elements,
                    &bindings.duplicate_elements,
                    &bindings.unique_deployments,
                    &bindings.duplicate_deployments,
                )
            }
            ReferenceTargetHint::Deployment => resolve_reference_against_binding_table(
                &contextual_raw_text,
                &bindings.unique_deployments,
                &bindings.duplicate_deployments,
            ),
            ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => {
                ReferenceResolutionStatus::UnresolvedNoMatch
            }
        };
        if status != ReferenceResolutionStatus::UnresolvedNoMatch {
            return status;
        }
    }

    ReferenceResolutionStatus::UnresolvedNoMatch
}

fn contextual_reference_prefixes(
    symbols: &[Symbol],
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
    target_hint: ReferenceTargetHint,
) -> Vec<String> {
    match target_hint {
        ReferenceTargetHint::Element => contextual_prefixes(
            symbols,
            containing_symbol,
            mode,
            &[CanonicalBindingKind::Element],
        ),
        ReferenceTargetHint::ElementOrDeployment => contextual_prefixes(
            symbols,
            containing_symbol,
            mode,
            &[
                CanonicalBindingKind::Element,
                CanonicalBindingKind::Deployment,
            ],
        ),
        ReferenceTargetHint::Deployment => contextual_prefixes(
            symbols,
            containing_symbol,
            mode,
            &[CanonicalBindingKind::Deployment],
        ),
        ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => {
            Vec::new()
        }
    }
}

fn contextual_selector_prefixes(
    symbols: &[Symbol],
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
) -> Vec<String> {
    contextual_prefixes(
        symbols,
        containing_symbol,
        mode,
        &[
            CanonicalBindingKind::Element,
            CanonicalBindingKind::Deployment,
        ],
    )
}

fn contextual_prefixes(
    symbols: &[Symbol],
    containing_symbol: SymbolId,
    mode: ElementIdentifierMode,
    binding_kinds: &[CanonicalBindingKind],
) -> Vec<String> {
    // Symbol-context references and `!element` selectors both walk outward
    // through the same ancestor chain. The only difference is whether one pass
    // should consider element bindings, deployment bindings, or both.
    let mut prefixes = Vec::new();
    let mut current = Some(containing_symbol);

    while let Some(symbol_id) = current {
        let symbol = symbols
            .get(symbol_id.0)
            .expect("BUG: contextual prefix symbol should exist");
        for binding_kind in binding_kinds {
            if let Some(prefix) = canonical_binding_key(symbols, symbol_id, mode, *binding_kind) {
                prefixes.push(prefix);
            }
        }
        current = symbol.parent;
    }

    prefixes
}

fn resolve_reference_with_selector_context(
    document: &WorkspaceSemanticDocumentFacts,
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    let Some(directive) = enclosing_element_directive(document, reference.span) else {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    };
    if !matches!(
        directive.target.value_kind,
        DirectiveValueKind::BareValue | DirectiveValueKind::Identifier
    ) {
        return ReferenceResolutionStatus::UnresolvedNoMatch;
    }

    for selector_target in element_selector_target_candidates(document, directive, bindings) {
        let contextual_raw_text = format!("{selector_target}.{}", reference.raw_text);
        let status = match reference.target_hint {
            ReferenceTargetHint::Element => resolve_reference_against_element_table(
                &contextual_raw_text,
                &bindings.unique_elements,
                &bindings.duplicate_elements,
            ),
            ReferenceTargetHint::ElementOrDeployment => {
                resolve_reference_against_element_or_deployment_tables(
                    &contextual_raw_text,
                    &bindings.unique_elements,
                    &bindings.duplicate_elements,
                    &bindings.unique_deployments,
                    &bindings.duplicate_deployments,
                )
            }
            ReferenceTargetHint::Deployment => resolve_reference_against_binding_table(
                &contextual_raw_text,
                &bindings.unique_deployments,
                &bindings.duplicate_deployments,
            ),
            ReferenceTargetHint::Relationship | ReferenceTargetHint::ElementOrRelationship => {
                ReferenceResolutionStatus::UnresolvedNoMatch
            }
        };
        if status != ReferenceResolutionStatus::UnresolvedNoMatch {
            return status;
        }
    }

    ReferenceResolutionStatus::UnresolvedNoMatch
}

fn enclosing_element_directive(
    document: &WorkspaceSemanticDocumentFacts,
    span: TextSpan,
) -> Option<&ElementDirectiveFact> {
    document
        .element_directives
        .iter()
        .filter(|directive| span_within(directive.span, span))
        .min_by_key(|directive| directive.span.end_byte - directive.span.start_byte)
}

fn element_selector_target_candidates(
    document: &WorkspaceSemanticDocumentFacts,
    directive: &ElementDirectiveFact,
    bindings: &WorkspaceBindingTables,
) -> Vec<String> {
    let raw_text = directive.target.normalized_text.as_str();
    let mut candidates = vec![raw_text.to_owned()];

    let Some(mode) = bindings.element_modes.get(&document.document_id).copied() else {
        return candidates;
    };
    let Some(containing_symbol) = enclosing_symbol_for_span(document, directive.span) else {
        return candidates;
    };

    for prefix in contextual_selector_prefixes(&document.symbols, containing_symbol, mode) {
        candidates.push(format!("{prefix}.{raw_text}"));
    }

    candidates
}

fn selector_segment_index(target: &ValueFact, reference_span: TextSpan) -> Option<usize> {
    let prefix_end = reference_span
        .end_byte
        .checked_sub(target.span.start_byte)?;
    let prefix = target.normalized_text.get(..prefix_end)?;
    Some(prefix.split('.').count().saturating_sub(1))
}

fn selector_segment_handle(
    target: &SymbolHandle,
    segment_index: usize,
    documents_by_id: &BTreeMap<DocumentId, &WorkspaceSemanticDocumentFacts>,
) -> Option<SymbolHandle> {
    let target_document = documents_by_id.get(target.document())?;
    let path = canonical_symbol_path(&target_document.symbols, target.symbol_id())?;
    let symbol_id = *path.get(segment_index)?;
    Some(SymbolHandle::new(target.document().clone(), symbol_id))
}
