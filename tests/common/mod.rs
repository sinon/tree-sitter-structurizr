#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser, Point, Tree};

#[derive(Debug)]
pub struct FixtureCase {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
    pub expectation: FixtureExpectation,
}

impl FixtureCase {
    pub fn snapshot_name(&self) -> String {
        format!("fixture__{}", self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureExpectation {
    ParseOk,
    ParseError,
}

pub fn parse(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_structurizr::LANGUAGE.into())
        .expect("Error loading Structurizr parser");
    parser
        .parse(source, None)
        .expect("Parser returned no tree for fixture")
}

pub fn tree_sexp(tree: &Tree) -> String {
    format_tree_sexp(&tree.root_node().to_sexp())
}

#[derive(Debug, Clone)]
pub struct ParseIssue {
    pub kind: &'static str,
    pub node_kind: String,
    pub start: Point,
    pub end: Point,
    pub text: String,
}

pub fn collect_parse_issues(tree: &Tree, source: &str) -> Vec<ParseIssue> {
    let mut issues = Vec::new();
    collect_node_issues(tree.root_node(), source, &mut issues);
    issues
}

pub fn assert_no_errors(label: &str, tree: &Tree, source: &str) {
    assert!(
        !tree.root_node().has_error(),
        "expected `{label}` to parse without errors\nsource:\n{source}\n\nsexp:\n{}",
        tree_sexp(tree)
    );
}

pub fn assert_has_errors(label: &str, tree: &Tree, source: &str) {
    assert!(
        tree.root_node().has_error(),
        "expected `{label}` to contain parse errors while coverage is pending\nsource:\n{source}\n\nsexp:\n{}",
        tree_sexp(tree)
    );
}

pub fn load_fixture(path: impl AsRef<Path>) -> FixtureCase {
    let path = path.as_ref().to_path_buf();
    FixtureCase {
        name: relative_fixture_name(&path),
        source: fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read fixture `{}`: {error}", path.display())),
        expectation: fixture_expectation(&path),
        path,
    }
}

fn relative_fixture_name(path: &Path) -> String {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let relative = path
        .strip_prefix(&fixture_root)
        .or_else(|_| path.strip_prefix("tests/fixtures"))
        .unwrap_or_else(|_| panic!("fixture path should live under tests/fixtures: {}", path.display()))
        .with_extension("");

    relative
        .components()
        .map(|component| fixture_name_component(&component.as_os_str().to_string_lossy()))
        .collect::<Vec<_>>()
        .join("__")
}

fn fixture_name_component(component: &str) -> String {
    if let Some(base) = component.strip_suffix("-ok") {
        format!("{}_ok", normalize_fixture_component(base))
    } else if let Some(base) = component.strip_suffix("-err") {
        format!("{}_err", normalize_fixture_component(base))
    } else {
        normalize_fixture_component(component)
    }
}

fn normalize_fixture_component(component: &str) -> String {
    component.replace(['-', '.'], "_")
}

fn fixture_expectation(path: &Path) -> FixtureExpectation {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_else(|| panic!("fixture path should have valid utf-8 stem: {}", path.display()));

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

fn collect_node_issues(node: Node, source: &str, issues: &mut Vec<ParseIssue>) {
    if node.is_error() || node.is_missing() {
        issues.push(ParseIssue {
            kind: if node.is_missing() { "MISSING" } else { "ERROR" },
            node_kind: node.kind().to_string(),
            start: node.start_position(),
            end: node.end_position(),
            text: issue_text(node, source),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_node_issues(child, source, issues);
    }
}

fn issue_text(node: Node, source: &str) -> String {
    let bytes = source.as_bytes();
    let raw = if node.start_byte() < node.end_byte() {
        node.utf8_text(bytes).unwrap_or("")
    } else {
        context_excerpt(source, node.start_byte())
    };

    let squashed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if squashed.is_empty() {
        "<empty>".to_string()
    } else {
        squashed
    }
}

fn context_excerpt(source: &str, byte: usize) -> &str {
    let start = byte.saturating_sub(30);
    let end = (byte + 30).min(source.len());
    &source[start..end]
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
