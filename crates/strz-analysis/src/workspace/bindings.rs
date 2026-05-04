fn canonical_symbol_path(symbols: &[Symbol], target: SymbolId) -> Option<Vec<SymbolId>> {
    let binding_family = canonical_binding_family(symbols.get(target.0)?.kind)?;
    let mut path = vec![target];
    let mut parent = symbols.get(target.0)?.parent;

    while let Some(parent_id) = parent {
        let ancestor = symbols.get(parent_id.0)?;
        if !binding_family_matches(binding_family, ancestor.kind) {
            return None;
        }
        path.push(parent_id);
        parent = ancestor.parent;
    }

    path.reverse();
    Some(path)
}

// Canonical binding-key construction, identifier-mode handling, and low-level
// binding-table lookups shared by index and reference resolution.

#[derive(Clone, Copy)]
enum CanonicalBindingFamily {
    Element,
    Deployment,
}

const fn canonical_binding_family(kind: SymbolKind) -> Option<CanonicalBindingFamily> {
    match kind {
        SymbolKind::Person
        | SymbolKind::SoftwareSystem
        | SymbolKind::Container
        | SymbolKind::Component => Some(CanonicalBindingFamily::Element),
        SymbolKind::DeploymentEnvironment
        | SymbolKind::DeploymentNode
        | SymbolKind::InfrastructureNode
        | SymbolKind::ContainerInstance
        | SymbolKind::SoftwareSystemInstance => Some(CanonicalBindingFamily::Deployment),
        SymbolKind::Relationship => None,
    }
}

const fn binding_family_matches(family: CanonicalBindingFamily, kind: SymbolKind) -> bool {
    match family {
        CanonicalBindingFamily::Element => matches!(
            kind,
            SymbolKind::Person
                | SymbolKind::SoftwareSystem
                | SymbolKind::Container
                | SymbolKind::Component
        ),
        CanonicalBindingFamily::Deployment => matches!(
            kind,
            SymbolKind::DeploymentEnvironment
                | SymbolKind::DeploymentNode
                | SymbolKind::InfrastructureNode
                | SymbolKind::ContainerInstance
                | SymbolKind::SoftwareSystemInstance
        ),
    }
}

const fn span_within(outer: TextSpan, inner: TextSpan) -> bool {
    outer.start_byte <= inner.start_byte && inner.end_byte <= outer.end_byte
}

fn resolve_reference_against_target_hint(
    reference: &Reference,
    bindings: &WorkspaceBindingTables,
) -> ReferenceResolutionStatus {
    match reference.target_hint {
        ReferenceTargetHint::Element => resolve_reference_against_element_table(
            &reference.raw_text,
            &bindings.unique_elements,
            &bindings.duplicate_elements,
        ),
        ReferenceTargetHint::ElementOrDeployment => {
            resolve_reference_against_element_or_deployment_tables(
                &reference.raw_text,
                &bindings.unique_elements,
                &bindings.duplicate_elements,
                &bindings.unique_deployments,
                &bindings.duplicate_deployments,
            )
        }
        ReferenceTargetHint::Deployment => resolve_reference_against_binding_table(
            &reference.raw_text,
            &bindings.unique_deployments,
            &bindings.duplicate_deployments,
        ),
        ReferenceTargetHint::Relationship => resolve_reference_against_binding_table(
            &reference.raw_text,
            &bindings.unique_relationships,
            &bindings.duplicate_relationships,
        ),
        ReferenceTargetHint::ElementOrRelationship => resolve_view_include_reference(
            &reference.raw_text,
            &bindings.unique_elements,
            &bindings.duplicate_elements,
            &bindings.unique_relationships,
            &bindings.duplicate_relationships,
        ),
    }
}

fn resolve_reference_against_element_table(
    raw_text: &str,
    unique_element_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_element_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
) -> ReferenceResolutionStatus {
    // Keep the element-flavoured wrapper even though it currently delegates
    // directly so the call sites still read in terms of binding families rather
    // than raw table plumbing.
    resolve_reference_against_binding_table(
        raw_text,
        unique_element_bindings,
        duplicate_element_bindings,
    )
}

fn resolve_reference_against_element_or_deployment_tables(
    raw_text: &str,
    unique_element_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_element_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
    unique_deployment_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_deployment_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
) -> ReferenceResolutionStatus {
    if duplicate_element_bindings.contains_key(raw_text)
        || duplicate_deployment_bindings.contains_key(raw_text)
    {
        return ReferenceResolutionStatus::AmbiguousDuplicateBinding;
    }

    match (
        unique_element_bindings.get(raw_text),
        unique_deployment_bindings.get(raw_text),
    ) {
        (Some(_), Some(_)) => ReferenceResolutionStatus::AmbiguousDuplicateBinding,
        (Some(handle), None) | (None, Some(handle)) => {
            ReferenceResolutionStatus::Resolved(handle.clone())
        }
        (None, None) => ReferenceResolutionStatus::UnresolvedNoMatch,
    }
}

fn resolve_reference_against_binding_table(
    raw_text: &str,
    unique_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
) -> ReferenceResolutionStatus {
    if duplicate_bindings.contains_key(raw_text) {
        return ReferenceResolutionStatus::AmbiguousDuplicateBinding;
    }

    unique_bindings.get(raw_text).cloned().map_or(
        ReferenceResolutionStatus::UnresolvedNoMatch,
        ReferenceResolutionStatus::Resolved,
    )
}

fn resolve_view_include_reference(
    raw_text: &str,
    unique_element_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_element_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
    unique_relationship_bindings: &BTreeMap<String, SymbolHandle>,
    duplicate_relationship_bindings: &BTreeMap<String, Vec<SymbolHandle>>,
) -> ReferenceResolutionStatus {
    if duplicate_element_bindings.contains_key(raw_text)
        || duplicate_relationship_bindings.contains_key(raw_text)
    {
        return ReferenceResolutionStatus::AmbiguousDuplicateBinding;
    }

    match (
        unique_element_bindings.get(raw_text),
        unique_relationship_bindings.get(raw_text),
    ) {
        (Some(_), Some(_)) => ReferenceResolutionStatus::AmbiguousElementVsRelationship,
        (Some(symbol), None) | (None, Some(symbol)) => {
            ReferenceResolutionStatus::Resolved(symbol.clone())
        }
        (None, None) => ReferenceResolutionStatus::UnresolvedNoMatch,
    }
}

fn effective_element_identifier_mode(
    document: &WorkspaceSemanticDocumentFacts,
    inherited_workspace_mode: Option<&IdentifierMode>,
) -> ElementIdentifierMode {
    effective_element_identifier_mode_from_facts(
        &document.identifier_modes,
        inherited_workspace_mode,
    )
}

/// Derives the bounded element-identifier mode from raw directive facts.
///
/// Both workspace indexes and snapshot-only LSP helpers rely on this
/// precedence, so keeping it shared prevents drift between read-only features
/// and edit planning.
pub fn effective_element_identifier_mode_from_facts(
    identifier_modes: &[IdentifierModeFact],
    inherited_workspace_mode: Option<&IdentifierMode>,
) -> ElementIdentifierMode {
    match document_model_identifier_mode(identifier_modes)
        .or_else(|| document_workspace_identifier_mode(identifier_modes))
        .or_else(|| inherited_workspace_mode.cloned())
    {
        Some(IdentifierMode::Hierarchical) => ElementIdentifierMode::Hierarchical,
        Some(IdentifierMode::Flat) | None => ElementIdentifierMode::Flat,
        Some(IdentifierMode::Other(_)) => ElementIdentifierMode::Deferred,
    }
}

fn document_model_identifier_mode(
    identifier_modes: &[IdentifierModeFact],
) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(identifier_modes, &DirectiveContainer::Model)
}

fn document_workspace_identifier_mode(
    identifier_modes: &[IdentifierModeFact],
) -> Option<IdentifierMode> {
    last_identifier_mode_for_container(identifier_modes, &DirectiveContainer::Workspace)
}

fn last_identifier_mode_for_container(
    identifier_modes: &[IdentifierModeFact],
    container: &DirectiveContainer,
) -> Option<IdentifierMode> {
    identifier_modes
        .iter()
        .rev()
        .find(|fact| fact.container == *container)
        .map(|fact| fact.mode.clone())
}

#[derive(Clone, Copy)]
enum CanonicalBindingKind {
    Element,
    Deployment,
}

impl CanonicalBindingKind {
    const fn allows_ancestor(self, kind: SymbolKind) -> bool {
        match self {
            Self::Element => matches!(
                kind,
                SymbolKind::Person
                    | SymbolKind::SoftwareSystem
                    | SymbolKind::Container
                    | SymbolKind::Component
            ),
            Self::Deployment => matches!(
                kind,
                SymbolKind::DeploymentEnvironment
                    | SymbolKind::DeploymentNode
                    | SymbolKind::InfrastructureNode
                    | SymbolKind::ContainerInstance
                    | SymbolKind::SoftwareSystemInstance
            ),
        }
    }
}

fn canonical_binding_key(
    symbols: &[Symbol],
    symbol_id: SymbolId,
    mode: ElementIdentifierMode,
    binding_kind: CanonicalBindingKind,
) -> Option<String> {
    let symbol = symbols.get(symbol_id.0)?;
    let binding_name = symbol.binding_name.as_deref()?;

    match mode {
        ElementIdentifierMode::Flat => Some(binding_name.to_owned()),
        ElementIdentifierMode::Deferred => None,
        ElementIdentifierMode::Hierarchical => {
            let mut segments = vec![binding_name.to_owned()];
            let mut parent = symbol.parent;

            while let Some(parent_id) = parent {
                let ancestor = symbols.get(parent_id.0)?;
                if !binding_kind.allows_ancestor(ancestor.kind) {
                    // Once the ancestor chain stops describing one canonical
                    // element/deployment path, drop the whole hierarchical key
                    // instead of emitting a truncated binding that would collide
                    // with a different legitimate declaration.
                    return None;
                }

                let ancestor_binding = ancestor.binding_name.as_deref()?;
                segments.push(ancestor_binding.to_owned());
                parent = ancestor.parent;
            }

            segments.reverse();
            Some(segments.join("."))
        }
    }
}

/// Returns the exact reference key for an element symbol under the supplied identifier mode.
#[must_use]
pub fn canonical_element_binding_key(
    symbols: &[Symbol],
    symbol_id: SymbolId,
    mode: ElementIdentifierMode,
) -> Option<String> {
    let symbol = symbols.get(symbol_id.0)?;
    if !matches!(
        symbol.kind,
        SymbolKind::Person
            | SymbolKind::SoftwareSystem
            | SymbolKind::Container
            | SymbolKind::Component
    ) {
        return None;
    }

    canonical_binding_key(symbols, symbol_id, mode, CanonicalBindingKind::Element)
}

/// Returns the exact reference key for a deployment symbol under the supplied identifier mode.
#[must_use]
pub fn canonical_deployment_binding_key(
    symbols: &[Symbol],
    symbol_id: SymbolId,
    mode: ElementIdentifierMode,
) -> Option<String> {
    let symbol = symbols.get(symbol_id.0)?;
    if !matches!(
        symbol.kind,
        SymbolKind::DeploymentEnvironment
            | SymbolKind::DeploymentNode
            | SymbolKind::InfrastructureNode
            | SymbolKind::ContainerInstance
            | SymbolKind::SoftwareSystemInstance
    ) {
        return None;
    }

    canonical_binding_key(symbols, symbol_id, mode, CanonicalBindingKind::Deployment)
}
