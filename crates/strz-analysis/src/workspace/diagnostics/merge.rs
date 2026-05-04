// Final semantic-diagnostic merging for documents that participate in multiple
// candidate workspace instances with potentially different outcomes.

fn merge_semantic_diagnostics(
    workspace_indexes: &[WorkspaceIndex],
    document_instances: &BTreeMap<DocumentId, Vec<WorkspaceInstanceId>>,
) -> Vec<RuledDiagnostic> {
    // A document can participate in multiple candidate workspace instances.
    // Exact agreement keeps the original diagnostic. Any disagreement becomes a
    // warning with unioned related context so shared fragments do not hide
    // context-specific issues or publish one root's view as unconditional truth.
    let mut diagnostic_instances =
        BTreeMap::<DocumentId, BTreeMap<RuledDiagnostic, BTreeSet<WorkspaceInstanceId>>>::new();

    for workspace_index in workspace_indexes {
        let mut per_document = BTreeMap::<DocumentId, BTreeSet<RuledDiagnostic>>::new();

        for diagnostic in workspace_index.semantic_diagnostics() {
            per_document
                .entry(
                    diagnostic
                        .document()
                        .expect("semantic diagnostics should carry documents")
                        .clone(),
                )
                .or_default()
                .insert(diagnostic.clone());
        }

        for (document, diagnostics) in per_document {
            let counts = diagnostic_instances.entry(document).or_default();
            for diagnostic in diagnostics {
                counts
                    .entry(diagnostic)
                    .or_default()
                    .insert(workspace_index.id());
            }
        }
    }

    let mut merged = Vec::new();
    for (document, instances) in document_instances {
        let Some(counts) = diagnostic_instances.get(document) else {
            continue;
        };

        let mut primary_groups = BTreeMap::<
            SemanticDiagnosticMergeKey,
            Vec<(&RuledDiagnostic, &BTreeSet<WorkspaceInstanceId>)>,
        >::new();
        for (diagnostic, reported_instances) in counts {
            if reported_instances.len() == instances.len() {
                merged.push(diagnostic.clone());
            } else {
                primary_groups
                    .entry(SemanticDiagnosticMergeKey::from_diagnostic(diagnostic))
                    .or_default()
                    .push((diagnostic, reported_instances));
            }
        }

        for variants in primary_groups.values() {
            let mut reported_instances = BTreeSet::new();
            for (_, variant_instances) in variants {
                reported_instances.extend(variant_instances.iter().copied());
            }

            let representative = variants[0].0;
            let mut diagnostic = RuledDiagnostic::multi_context_disagreement(
                document,
                representative.message(),
                reported_instances.len(),
                instances.len(),
                representative.span(),
            );
            diagnostic.diagnostic.annotations = merged_annotations(variants);
            merged.push(diagnostic);
        }
    }

    sort_semantic_diagnostics(&mut merged);
    merged
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticDiagnosticMergeKey {
    rule: RuleId,
    span: TextSpan,
    message: String,
    target_text: Option<String>,
    value_span: Option<TextSpan>,
}

impl SemanticDiagnosticMergeKey {
    fn from_diagnostic(diagnostic: &RuledDiagnostic) -> Self {
        Self {
            rule: diagnostic.rule,
            span: diagnostic.span(),
            message: diagnostic.message().to_owned(),
            target_text: diagnostic.target_text().map(str::to_owned),
            value_span: diagnostic.value_span(),
        }
    }
}

fn merged_annotations(
    variants: &[(&RuledDiagnostic, &BTreeSet<WorkspaceInstanceId>)],
) -> Vec<Annotation> {
    variants
        .iter()
        .flat_map(|(diagnostic, _)| diagnostic.annotations().iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn sort_semantic_diagnostics(diagnostics: &mut [RuledDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.document()
            .cmp(&right.document())
            .then_with(|| left.span().start_byte.cmp(&right.span().start_byte))
            .then_with(|| left.rule.cmp(&right.rule))
            .then_with(|| left.message().cmp(right.message()))
    });
}
