use std::fmt::Write as _;

use proptest::collection::vec;
use proptest::prelude::any;
use proptest::strategy::{BoxedStrategy, Strategy};
use proptest::string::string_regex;
use proptest::test_runner::{Config, TestCaseError};
use structurizr_analysis::{DocumentInput, DocumentSnapshot, TextSpan, analyze_document};

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

fn analysis_source() -> BoxedStrategy<String> {
    proptest::prop_oneof![
        3 => arbitrary_utf8_source(),
        1 => valid_workspace_source(),
    ]
    .boxed()
}

fn analyze_source(source: &str) -> DocumentSnapshot {
    analyze_document(DocumentInput::new("generated.dsl", source))
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

    assert_span_within_source(
        TextSpan::from_node(snapshot.tree().root_node()),
        source,
        "root node",
    )?;

    for (index, diagnostic) in snapshot.syntax_diagnostics().iter().enumerate() {
        assert_span_within_source(
            diagnostic.span,
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

proptest::proptest! {
    #![proptest_config(Config {
        cases: 64,
        failure_persistence: None,
        ..Config::default()
    })]

    #[test]
    fn analysis_handles_generated_sources_without_panicking(source in analysis_source()) {
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
        let snapshot = analyze_source(&source);
        assert_snapshot_spans_within_source(&snapshot)?;
    }

    #[test]
    fn generated_valid_workspaces_analyze_without_syntax_errors(source in valid_workspace_source()) {
        let snapshot = analyze_source(&source);

        proptest::prop_assert!(
            !snapshot.has_syntax_errors(),
            "expected generated workspace to analyze without syntax errors\nsource:\n{source}\n\nsexp:\n{}",
            snapshot.tree().root_node().to_sexp(),
        );
    }
}
