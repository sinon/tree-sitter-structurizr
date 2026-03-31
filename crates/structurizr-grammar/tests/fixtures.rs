use std::fs;
use std::path::Path;
use std::path::PathBuf;

use rstest::rstest;
use tree_sitter::{Parser, Tree};

macro_rules! set_snapshot_suffix {
    ($($expr:expr),* $(,)?) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_suffix(format!($($expr),*));
        let _guard = settings.bind_to_scope();
    };
}

#[derive(Debug)]
struct FixtureCase {
    name: String,
    source: String,
    expectation: FixtureExpectation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureExpectation {
    ParseOk,
    ParseError,
}

#[rstest]
fn fixtures_match_expected_parse_outcomes(#[files("../../fixtures/**/*.dsl")] path: PathBuf) {
    let fixture = load_fixture(&path);
    let tree = parse(&fixture.source);

    match fixture.expectation {
        FixtureExpectation::ParseOk => {
            assert_no_errors(&fixture.name, &tree, &fixture.source);
        }
        FixtureExpectation::ParseError => {
            assert_has_errors(&fixture.name, &tree, &fixture.source);
        }
    }

    set_snapshot_suffix!("{}", fixture.name);
    insta::assert_snapshot!("fixture", tree_sexp(&tree));
}

fn load_fixture(path: impl AsRef<Path>) -> FixtureCase {
    let path = path.as_ref();
    FixtureCase {
        name: relative_fixture_name(path),
        source: fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read fixture `{}`: {error}", path.display())),
        expectation: fixture_expectation(path),
    }
}

fn parse(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_structurizr::LANGUAGE.into())
        .expect("Structurizr language should load");
    parser
        .parse(source, None)
        .expect("fixture source should produce a tree")
}

fn assert_no_errors(label: &str, tree: &Tree, source: &str) {
    assert!(
        !tree.root_node().has_error(),
        "expected `{label}` to parse without errors\nsource:\n{source}\n\nsexp:\n{}",
        tree_sexp(tree)
    );
}

fn assert_has_errors(label: &str, tree: &Tree, source: &str) {
    assert!(
        tree.root_node().has_error(),
        "expected `{label}` to contain parse errors while coverage is pending\nsource:\n{source}\n\nsexp:\n{}",
        tree_sexp(tree)
    );
}

fn relative_fixture_name(path: &Path) -> String {
    let fixture_root = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures"))
        .unwrap_or_else(|error| panic!("failed to canonicalize fixture root: {error}"));
    let fixture_path = fs::canonicalize(path).unwrap_or_else(|error| {
        panic!(
            "failed to canonicalize fixture `{}`: {error}",
            path.display()
        )
    });
    let relative = fixture_path
        .strip_prefix(&fixture_root)
        .unwrap_or_else(|_| {
            panic!(
                "fixture path should live under fixtures: {}",
                path.display()
            )
        })
        .with_extension("");

    relative
        .components()
        .map(|component| fixture_name_component(&component.as_os_str().to_string_lossy()))
        .collect::<Vec<_>>()
        .join("__")
}

fn fixture_name_component(component: &str) -> String {
    component.strip_suffix("-ok").map_or_else(
        || {
            component.strip_suffix("-err").map_or_else(
                || normalize_fixture_component(component),
                |base| format!("{}_err", normalize_fixture_component(base)),
            )
        },
        |base| format!("{}_ok", normalize_fixture_component(base)),
    )
}

fn normalize_fixture_component(component: &str) -> String {
    component.replace(['-', '.'], "_")
}

fn fixture_expectation(path: &Path) -> FixtureExpectation {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_else(|| {
            panic!(
                "fixture path should have valid utf-8 stem: {}",
                path.display()
            )
        });

    if stem.ends_with("-ok") {
        FixtureExpectation::ParseOk
    } else if stem.ends_with("-err") {
        FixtureExpectation::ParseError
    } else {
        panic!(
            "fixture name must end with `-ok.dsl` or `-err.dsl`: {}",
            path.display()
        );
    }
}

fn tree_sexp(tree: &Tree) -> String {
    format_tree_sexp(&tree.root_node().to_sexp())
}

fn format_tree_sexp(sexp: &str) -> String {
    let mut formatted = String::new();
    let mut indent = 0usize;
    let mut i = 0usize;
    let bytes = sexp.as_bytes();

    while i < bytes.len() {
        match bytes[i] as char {
            '(' => {
                if !formatted.is_empty() {
                    formatted.push('\n');
                }
                formatted.push_str(&"  ".repeat(indent));
                formatted.push('(');
                indent += 1;
                i += 1;

                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch == '(' || ch == ')' || ch.is_whitespace() {
                        break;
                    }
                    formatted.push(ch);
                    i += 1;
                }
            }
            ')' => {
                indent = indent.saturating_sub(1);
                formatted.push(')');
                i += 1;
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            _ => {
                formatted.push(' ');
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch == '(' || ch == ')' || ch.is_whitespace() {
                        break;
                    }
                    formatted.push(ch);
                    i += 1;
                }
            }
        }
    }

    formatted
}
