use indoc::indoc;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

const GENERATED_GRAMMAR_JSON: &str = include_str!("../src/grammar.json");
const HIGHLIGHTS_QUERY: &str = include_str!("../queries/highlights.scm");

fn parse(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_structurizr::LANGUAGE.into())
        .expect("Structurizr language should load");
    parser
        .parse(source, None)
        .expect("source should produce a tree")
}

fn highlight_captures(source: &str) -> Vec<(String, String)> {
    let tree = parse(source);
    assert!(
        !tree.root_node().has_error(),
        "expected highlight fixture to parse without errors\nsource:\n{source}\n\nsexp:\n{}",
        tree.root_node().to_sexp()
    );

    let language = tree_sitter_structurizr::LANGUAGE.into();
    let query = Query::new(&language, HIGHLIGHTS_QUERY).expect("highlight query should compile");
    let capture_names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut captures = Vec::new();

    let mut query_matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    query_matches.advance();

    while let Some(query_match) = query_matches.get() {
        for capture in query_match.captures {
            let text = capture
                .node
                .utf8_text(source.as_bytes())
                .expect("capture should be utf-8");
            captures.push((
                capture_names[capture.index as usize].to_string(),
                text.to_string(),
            ));
        }
        query_matches.advance();
    }

    captures
}

#[test]
fn generated_grammar_declares_identifier_as_the_word_token() {
    // The keyword-bleed regression shows up in downstream highlighters rather than
    // in raw query captures, so lock in the structural precondition directly.
    assert!(
        GENERATED_GRAMMAR_JSON.contains("\"word\": \"identifier\""),
        "expected generated grammar JSON to declare `identifier` as the word token"
    );
}

#[test]
fn url_statements_highlight_unquoted_values_as_urls() {
    // `url https://...` still parses the value as `bare_value`, so the query must
    // classify that shape explicitly instead of leaving it to fallback styling.
    let source = indoc! {r#"
        workspace {
          model {
            system = softwareSystem "System" {
              api = container "API" {
                securityComponent = component "Security" {
                  url https://example.com/docs
                }
              }
            }
          }
        }
    "#};

    let captures = highlight_captures(source);

    assert!(
        captures
            .iter()
            .any(|(name, text)| name == "keyword" && text == "url"),
        "expected `url` to be highlighted as a keyword, got {captures:#?}"
    );
    assert!(
        captures.iter().any(|(name, text)| {
            name == "string.special.url" && text == "https://example.com/docs"
        }),
        "expected unquoted URL values to be highlighted as URLs, got {captures:#?}"
    );
}

#[test]
fn deployment_instances_highlight_identifiers_targets_groups_and_keywords() {
    let source = indoc! {r#"
        workspace {
          model {
            system = softwareSystem "System" {
              app = container "App"
            }

            live = deploymentEnvironment "Live" {
              blue = deploymentGroup "Blue"

              node = deploymentNode "Node" {
                systemInstance = softwareSystemInstance system
                canarySystem = softwareSystemInstance system blue "Canary"
                canaryApp = containerInstance app blue "Canary"
              }
            }
          }
        }
    "#};

    let captures = highlight_captures(source);

    for keyword in ["softwareSystemInstance", "containerInstance"] {
        assert!(
            captures
                .iter()
                .any(|(name, text)| name == "keyword" && text == keyword),
            "expected `{keyword}` to be highlighted as a keyword, got {captures:#?}"
        );
    }

    for ty in [
        "systemInstance",
        "canarySystem",
        "canaryApp",
        "system",
        "app",
    ] {
        assert!(
            captures.iter().any(|(name, text)| name == "type" && text == ty),
            "expected `{ty}` to be highlighted as a type, got {captures:#?}"
        );
    }

    let blue_type_count = captures
        .iter()
        .filter(|(name, text)| name == "type" && text == "blue")
        .count();
    assert_eq!(
        blue_type_count, 3,
        "expected `blue` to be highlighted as a type in its declaration and both instance references, got {captures:#?}"
    );
}
