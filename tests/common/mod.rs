#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser, Point, Tree};

#[derive(Debug)]
pub struct FixtureCase {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
}

impl FixtureCase {
    pub fn snapshot_name(&self) -> String {
        format!("fixture__{}", self.name)
    }
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
    tree.root_node().to_sexp()
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

pub fn load_fixtures(root: impl AsRef<Path>) -> Vec<FixtureCase> {
    let root = root.as_ref();
    let mut paths = Vec::new();
    collect_dsl_files(root, &mut paths);
    paths.sort();

    paths.into_iter()
        .map(|path| FixtureCase {
            name: relative_fixture_name(&path),
            source: fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read fixture `{}`: {error}", path.display())),
            path,
        })
        .collect()
}

fn collect_dsl_files(root: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read fixture directory `{}`: {error}", root.display()))
    {
        let entry = entry.expect("failed to read fixture directory entry");
        let path = entry.path();

        if path.is_dir() {
            collect_dsl_files(&path, paths);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("dsl") {
            paths.push(path);
        }
    }
}

fn relative_fixture_name(path: &Path) -> String {
    path.strip_prefix("tests/fixtures")
        .expect("fixture path should live under tests/fixtures")
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().replace('-', "_"))
        .collect::<Vec<_>>()
        .join("__")
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
