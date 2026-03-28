use std::fmt::Write as _;

use proptest::collection::vec;
use proptest::prelude::any;
use proptest::strategy::Strategy;
use proptest::string::string_regex;
use proptest::test_runner::Config;
use tree_sitter::Parser;

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

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_structurizr::LANGUAGE.into())
        .expect("Structurizr language should load");
    parser
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

proptest::proptest! {
    #![proptest_config(Config {
        cases: 64,
        failure_persistence: None,
        ..Config::default()
    })]

    #[test]
    fn parser_handles_generated_utf8_without_panicking(source in arbitrary_utf8_source()) {
        let tree = parser()
            .parse(&source, None)
            .expect("generated source should always produce a tree");

        proptest::prop_assert!(tree.root_node().start_byte() <= tree.root_node().end_byte());
        proptest::prop_assert!(source.is_char_boundary(tree.root_node().start_byte()));
        proptest::prop_assert!(tree.root_node().end_byte() <= source.len());
        proptest::prop_assert!(source.is_char_boundary(tree.root_node().end_byte()));
    }

    #[test]
    fn valid_generated_workspaces_parse_without_error_nodes(source in valid_workspace_source()) {
        let tree = parser()
            .parse(&source, None)
            .expect("generated workspace should always produce a tree");

        proptest::prop_assert!(
            !tree.root_node().has_error(),
            "expected generated workspace to parse without errors\nsource:\n{source}\n\nsexp:\n{}",
            tree.root_node().to_sexp(),
        );
        proptest::prop_assert_eq!(tree.root_node().end_byte(), source.len());
    }
}
