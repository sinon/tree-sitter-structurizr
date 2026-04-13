use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use proptest::collection::vec;
use proptest::prelude::any;
use proptest::strategy::{BoxedStrategy, Just, Strategy};
use proptest::string::string_regex;
use proptest::test_runner::{Config, TestCaseError};
use strz_analysis::{DocumentAnalyzer, DocumentInput, DocumentSnapshot, TextSpan};

#[derive(Debug, Clone)]
struct GeneratedWorkspace {
    person_id: String,
    person_name: String,
    system_id: String,
    system_name: String,
    relationship_label: String,
    include_relationship: bool,
    include_views: bool,
}

#[derive(Debug, Clone, Copy)]
enum WorkspaceMutation {
    RemoveFinalBrace,
    AppendDanglingQuote,
    AppendDanglingBrace,
}

impl GeneratedWorkspace {
    fn render(&self) -> String {
        let mut source = format!(
            "workspace {{\n    model {{\n        {} = person \"{}\"\n        {} = softwareSystem \"{}\"\n",
            self.person_id, self.person_name, self.system_id, self.system_name
        );

        if self.include_relationship {
            writeln!(
                source,
                "        {} -> {} \"{}\"",
                self.person_id, self.system_id, self.relationship_label
            )
            .expect("writing to String should succeed");
        }

        source.push_str("    }\n");

        if self.include_views {
            source.push_str("    views {\n    }\n");
        }

        source.push_str("}\n");
        source
    }
}

fn arbitrary_utf8_source() -> impl Strategy<Value = String> {
    vec(any::<char>(), 0..128).prop_map(|chars| chars.into_iter().collect())
}

fn identifier() -> impl Strategy<Value = String> {
    string_regex("[a-z][a-z0-9_]{0,11}").expect("identifier regex should compile")
}

fn display_name() -> impl Strategy<Value = String> {
    string_regex("[A-Z][A-Za-z0-9 ]{0,15}").expect("display-name regex should compile")
}

fn relationship_label() -> impl Strategy<Value = String> {
    string_regex("[A-Z][A-Za-z0-9 ]{0,15}").expect("relationship-label regex should compile")
}

fn generated_workspace() -> impl Strategy<Value = GeneratedWorkspace> {
    (
        identifier(),
        display_name(),
        identifier(),
        display_name(),
        relationship_label(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_filter(
            "generated identifiers must differ",
            |(person_id, _, system_id, _, _, _, _)| person_id != system_id,
        )
        .prop_map(
            |(
                person_id,
                person_name,
                system_id,
                system_name,
                relationship_label,
                include_relationship,
                include_views,
            )| GeneratedWorkspace {
                person_id,
                person_name,
                system_id,
                system_name,
                relationship_label,
                include_relationship,
                include_views,
            },
        )
}

fn valid_workspace_source() -> impl Strategy<Value = String> {
    generated_workspace().prop_map(|workspace| workspace.render())
}

fn workspace_mutation() -> impl Strategy<Value = WorkspaceMutation> {
    proptest::prop_oneof![
        Just(WorkspaceMutation::RemoveFinalBrace),
        Just(WorkspaceMutation::AppendDanglingQuote),
        Just(WorkspaceMutation::AppendDanglingBrace),
    ]
}

fn invalid_workspace_source() -> impl Strategy<Value = String> {
    (valid_workspace_source(), workspace_mutation())
        .prop_map(|(source, mutation)| mutate_workspace_source(&source, mutation))
}

fn mutate_workspace_source(source: &str, mutation: WorkspaceMutation) -> String {
    match mutation {
        WorkspaceMutation::RemoveFinalBrace => source
            .strip_suffix("}\n")
            .map_or_else(|| source.to_owned(), |prefix| format!("{prefix}\n")),
        WorkspaceMutation::AppendDanglingQuote => format!("{source}\""),
        WorkspaceMutation::AppendDanglingBrace => format!("{source}{{"),
    }
}

fn analysis_source() -> BoxedStrategy<String> {
    proptest::prop_oneof![
        3 => arbitrary_utf8_source(),
        1 => valid_workspace_source(),
    ]
    .boxed()
}

fn analyze_source(source: &str) -> DocumentSnapshot {
    let mut analyzer = DocumentAnalyzer::new();
    analyzer.analyze(DocumentInput::new("generated.dsl", source))
}

fn proptest_config() -> Config {
    let mut config = Config::default();

    if env::var_os("PROPTEST_CASES").is_none() {
        config.cases = 64;
    }

    config
}

fn maybe_capture_source(test_name: &str, source: &str) {
    let Some(capture_dir) = env::var_os("STRUCTURIZR_PROPTEST_CAPTURE_DIR").map(PathBuf::from)
    else {
        return;
    };

    fs::create_dir_all(&capture_dir).expect("capture directory should create");
    fs::write(capture_dir.join(format!("{test_name}.dsl")), source)
        .expect("captured source should write");
}

fn assert_span_within_source(
    span: TextSpan,
    source: &str,
    label: &str,
) -> Result<(), TestCaseError> {
    proptest::prop_assert!(
        span.start_byte <= span.end_byte,
        "{label} should not invert byte offsets: {span:?}",
    );
    proptest::prop_assert!(
        span.end_byte <= source.len(),
        "{label} should stay within source bounds: {span:?} vs {} bytes",
        source.len(),
    );
    proptest::prop_assert!(
        source.is_char_boundary(span.start_byte),
        "{label} should start on a char boundary: {span:?}",
    );
    proptest::prop_assert!(
        source.is_char_boundary(span.end_byte),
        "{label} should end on a char boundary: {span:?}",
    );
    proptest::prop_assert!(
        span.start_point.row < span.end_point.row
            || (span.start_point.row == span.end_point.row
                && span.start_point.column <= span.end_point.column),
        "{label} should not invert text points: {span:?}",
    );

    Ok(())
}

fn assert_snapshot_spans_within_source(snapshot: &DocumentSnapshot) -> Result<(), TestCaseError> {
    let source = snapshot.source();

    assert_core_snapshot_spans_within_source(snapshot, source)?;
    assert_semantic_snapshot_spans_within_source(snapshot, source)?;

    Ok(())
}

fn assert_core_snapshot_spans_within_source(
    snapshot: &DocumentSnapshot,
    source: &str,
) -> Result<(), TestCaseError> {
    assert_span_within_source(
        TextSpan::from_node(snapshot.tree().root_node()),
        source,
        "root node",
    )?;

    for (index, diagnostic) in snapshot.syntax_diagnostics().iter().enumerate() {
        assert_span_within_source(
            diagnostic.span(),
            source,
            &format!("syntax diagnostic #{index}"),
        )?;
    }

    for (index, include) in snapshot.include_directives().iter().enumerate() {
        assert_span_within_source(include.span, source, &format!("include #{index} span"))?;
        assert_span_within_source(
            include.value_span,
            source,
            &format!("include #{index} value span"),
        )?;
    }

    for (index, constant) in snapshot.constant_definitions().iter().enumerate() {
        assert_span_within_source(constant.span, source, &format!("constant #{index} span"))?;
        assert_span_within_source(
            constant.name_span,
            source,
            &format!("constant #{index} name span"),
        )?;
        assert_span_within_source(
            constant.value_span,
            source,
            &format!("constant #{index} value span"),
        )?;
    }

    for (index, identifier_mode) in snapshot.identifier_modes().iter().enumerate() {
        assert_span_within_source(
            identifier_mode.span,
            source,
            &format!("identifier-mode #{index} span"),
        )?;
        assert_span_within_source(
            identifier_mode.value_span,
            source,
            &format!("identifier-mode #{index} value span"),
        )?;
    }

    for (index, symbol) in snapshot.symbols().iter().enumerate() {
        assert_span_within_source(symbol.span, source, &format!("symbol #{index} span"))?;
    }

    for (index, reference) in snapshot.references().iter().enumerate() {
        assert_span_within_source(reference.span, source, &format!("reference #{index} span"))?;
    }

    Ok(())
}

fn assert_semantic_snapshot_spans_within_source(
    snapshot: &DocumentSnapshot,
    source: &str,
) -> Result<(), TestCaseError> {
    for (index, section) in snapshot.workspace_sections().iter().enumerate() {
        assert_span_within_source(
            section.span,
            source,
            &format!("workspace section #{index} span"),
        )?;
    }

    for (index, scope) in snapshot.configuration_scopes().iter().enumerate() {
        assert_span_within_source(
            scope.span,
            source,
            &format!("configuration scope #{index} span"),
        )?;
        assert_span_within_source(
            scope.value.span,
            source,
            &format!("configuration scope #{index} value span"),
        )?;
    }

    for (index, property) in snapshot.property_facts().iter().enumerate() {
        assert_span_within_source(property.span, source, &format!("property #{index} span"))?;
        assert_span_within_source(
            property.name.span,
            source,
            &format!("property #{index} name span"),
        )?;
        assert_span_within_source(
            property.value.span,
            source,
            &format!("property #{index} value span"),
        )?;
    }

    for (index, resource) in snapshot.resource_directives().iter().enumerate() {
        assert_resource_directive_spans_within_source(resource, index, source)?;
    }

    for (index, directive) in snapshot.element_directives().iter().enumerate() {
        assert_span_within_source(
            directive.span,
            source,
            &format!("element directive #{index} span"),
        )?;
        assert_span_within_source(
            directive.target.span,
            source,
            &format!("element directive #{index} target span"),
        )?;
    }

    for (index, relationship) in snapshot.relationship_facts().iter().enumerate() {
        assert_span_within_source(
            relationship.span,
            source,
            &format!("relationship fact #{index} span"),
        )?;
        assert_span_within_source(
            relationship.source.span,
            source,
            &format!("relationship fact #{index} source span"),
        )?;
        assert_span_within_source(
            relationship.destination.span,
            source,
            &format!("relationship fact #{index} destination span"),
        )?;
        if let Some(description) = &relationship.description {
            assert_span_within_source(
                description.span,
                source,
                &format!("relationship fact #{index} description span"),
            )?;
        }
        if let Some(technology) = &relationship.technology {
            assert_span_within_source(
                technology.span,
                source,
                &format!("relationship fact #{index} technology span"),
            )?;
        }
    }

    for (index, view) in snapshot.view_facts().iter().enumerate() {
        assert_view_spans_within_source(view, index, source)?;
    }

    Ok(())
}

fn assert_resource_directive_spans_within_source(
    resource: &strz_analysis::ResourceDirectiveFact,
    index: usize,
    source: &str,
) -> Result<(), TestCaseError> {
    assert_span_within_source(
        resource.span,
        source,
        &format!("resource directive #{index} span"),
    )?;
    assert_span_within_source(
        resource.path.span,
        source,
        &format!("resource directive #{index} path span"),
    )?;
    if let Some(importer) = &resource.importer {
        assert_span_within_source(
            importer.span,
            source,
            &format!("resource directive #{index} importer span"),
        )?;
    }

    Ok(())
}

fn assert_view_spans_within_source(
    view: &strz_analysis::ViewFact,
    index: usize,
    source: &str,
) -> Result<(), TestCaseError> {
    assert_span_within_source(view.span, source, &format!("view #{index} span"))?;
    if let Some(body_span) = view.body_span {
        assert_span_within_source(body_span, source, &format!("view #{index} body span"))?;
    }

    for (label, value) in [
        ("key", view.key.as_ref()),
        ("scope", view.scope.as_ref()),
        ("environment", view.environment.as_ref()),
        ("base key", view.base_key.as_ref()),
        ("filter tags", view.filter_tags.as_ref()),
    ] {
        if let Some(value) = value {
            assert_span_within_source(value.span, source, &format!("view #{index} {label} span"))?;
        }
    }
    if let Some(auto_layout) = &view.auto_layout {
        assert_span_within_source(
            auto_layout.span,
            source,
            &format!("view #{index} auto layout span"),
        )?;
    }
    for (kind, values) in [
        ("include", &view.include_values),
        ("exclude", &view.exclude_values),
        ("animation", &view.animation_values),
    ] {
        for (value_index, value) in values.iter().enumerate() {
            assert_span_within_source(
                value.span,
                source,
                &format!("view #{index} {kind} #{value_index} span"),
            )?;
        }
    }
    for (step_index, step) in view.dynamic_steps.iter().enumerate() {
        assert_dynamic_step_spans_within_source(step, index, step_index, source)?;
    }
    for (source_index, image_source) in view.image_sources.iter().enumerate() {
        assert_image_source_spans_within_source(image_source, index, source_index, source)?;
    }

    Ok(())
}

fn assert_dynamic_step_spans_within_source(
    step: &strz_analysis::DynamicViewStepFact,
    view_index: usize,
    step_index: usize,
    source: &str,
) -> Result<(), TestCaseError> {
    match step {
        strz_analysis::DynamicViewStepFact::Relationship(step) => {
            assert_span_within_source(
                step.span,
                source,
                &format!("view #{view_index} dynamic relationship #{step_index} span"),
            )?;
            assert_span_within_source(
                step.source.span,
                source,
                &format!("view #{view_index} dynamic relationship #{step_index} source span"),
            )?;
            assert_span_within_source(
                step.destination.span,
                source,
                &format!("view #{view_index} dynamic relationship #{step_index} destination span"),
            )?;
            if let Some(description) = &step.description {
                assert_span_within_source(
                    description.span,
                    source,
                    &format!(
                        "view #{view_index} dynamic relationship #{step_index} description span"
                    ),
                )?;
            }
            if let Some(technology) = &step.technology {
                assert_span_within_source(
                    technology.span,
                    source,
                    &format!(
                        "view #{view_index} dynamic relationship #{step_index} technology span"
                    ),
                )?;
            }
        }
        strz_analysis::DynamicViewStepFact::RelationshipReference(step) => {
            assert_span_within_source(
                step.span,
                source,
                &format!("view #{view_index} dynamic reference #{step_index} span"),
            )?;
            assert_span_within_source(
                step.relationship.span,
                source,
                &format!("view #{view_index} dynamic reference #{step_index} relation span"),
            )?;
            assert_span_within_source(
                step.description.span,
                source,
                &format!("view #{view_index} dynamic reference #{step_index} description span"),
            )?;
        }
    }

    Ok(())
}

fn assert_image_source_spans_within_source(
    image_source: &strz_analysis::ImageSourceFact,
    view_index: usize,
    source_index: usize,
    source: &str,
) -> Result<(), TestCaseError> {
    assert_span_within_source(
        image_source.span,
        source,
        &format!("view #{view_index} image source #{source_index} span"),
    )?;
    assert_span_within_source(
        image_source.value.span,
        source,
        &format!("view #{view_index} image source #{source_index} value span"),
    )?;
    if let Some(format) = &image_source.format {
        assert_span_within_source(
            format.span,
            source,
            &format!("view #{view_index} image source #{source_index} format span"),
        )?;
    }

    Ok(())
}

proptest::proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn analysis_handles_generated_sources_without_panicking(source in analysis_source()) {
        maybe_capture_source("analysis_handles_generated_sources_without_panicking", &source);
        let snapshot = analyze_source(&source);

        proptest::prop_assert_eq!(snapshot.source(), source.as_str());
        proptest::prop_assert!(
            snapshot.tree().root_node().start_byte() <= snapshot.tree().root_node().end_byte()
        );
        proptest::prop_assert!(
            source.is_char_boundary(snapshot.tree().root_node().start_byte())
        );
        proptest::prop_assert!(snapshot.tree().root_node().end_byte() <= source.len());
        proptest::prop_assert!(source.is_char_boundary(snapshot.tree().root_node().end_byte()));
    }

    #[test]
    fn repeated_analysis_is_deterministic(source in analysis_source()) {
        maybe_capture_source("repeated_analysis_is_deterministic", &source);
        let first = analyze_source(&source);
        let second = analyze_source(&source);

        proptest::prop_assert_eq!(
            first.tree().root_node().to_sexp(),
            second.tree().root_node().to_sexp(),
        );
        proptest::prop_assert_eq!(first.is_workspace_entry(), second.is_workspace_entry());
        proptest::prop_assert_eq!(first.has_syntax_errors(), second.has_syntax_errors());
        proptest::prop_assert_eq!(first.syntax_diagnostics(), second.syntax_diagnostics());
        proptest::prop_assert_eq!(first.include_directives(), second.include_directives());
        proptest::prop_assert_eq!(first.constant_definitions(), second.constant_definitions());
        proptest::prop_assert_eq!(first.identifier_modes(), second.identifier_modes());
        proptest::prop_assert_eq!(first.symbols(), second.symbols());
        proptest::prop_assert_eq!(first.references(), second.references());
    }

    #[test]
    fn extracted_spans_stay_within_source_bounds(source in analysis_source()) {
        maybe_capture_source("extracted_spans_stay_within_source_bounds", &source);
        let snapshot = analyze_source(&source);
        assert_snapshot_spans_within_source(&snapshot)?;
    }

    #[test]
    fn generated_valid_workspaces_analyze_without_syntax_errors(source in valid_workspace_source()) {
        maybe_capture_source("generated_valid_workspaces_analyze_without_syntax_errors", &source);
        let snapshot = analyze_source(&source);

        proptest::prop_assert!(
            !snapshot.has_syntax_errors(),
            "expected generated workspace to analyze without syntax errors\nsource:\n{source}\n\nsexp:\n{}",
            snapshot.tree().root_node().to_sexp(),
        );
    }

    #[test]
    fn mutated_generated_workspaces_report_syntax_errors(source in invalid_workspace_source()) {
        maybe_capture_source("mutated_generated_workspaces_report_syntax_errors", &source);
        let snapshot = analyze_source(&source);

        proptest::prop_assert!(
            snapshot.has_syntax_errors(),
            "expected mutated workspace to report syntax errors\nsource:\n{source}\n\nsexp:\n{}",
            snapshot.tree().root_node().to_sexp(),
        );
        assert_snapshot_spans_within_source(&snapshot)?;
    }
}
