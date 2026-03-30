use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use proptest::collection::vec;
use proptest::prelude::any;
use proptest::strategy::{Just, Strategy};
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

proptest::proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn parser_handles_generated_utf8_without_panicking(source in arbitrary_utf8_source()) {
        maybe_capture_source("parser_handles_generated_utf8_without_panicking", &source);
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
        maybe_capture_source("valid_generated_workspaces_parse_without_error_nodes", &source);
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

    #[test]
    fn mutated_generated_workspaces_produce_error_nodes(source in invalid_workspace_source()) {
        maybe_capture_source("mutated_generated_workspaces_produce_error_nodes", &source);
        let tree = parser()
            .parse(&source, None)
            .expect("mutated workspace should always produce a tree");

        proptest::prop_assert!(
            tree.root_node().has_error(),
            "expected mutated workspace to produce parse errors\nsource:\n{source}\n\nsexp:\n{}",
            tree.root_node().to_sexp(),
        );
    }
}
