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

fn assert_has_capture(captures: &[(String, String)], name: &str, text: &str) {
    assert!(
        captures
            .iter()
            .any(|(capture_name, capture_text)| capture_name == name && capture_text == text),
        "expected `{text}` to be highlighted as `{name}`, got {captures:#?}"
    );
}

fn assert_all_have_capture(captures: &[(String, String)], name: &str, texts: &[&str]) {
    for text in texts {
        assert_has_capture(captures, name, text);
    }
}

const fn styles_highlight_fixture() -> &'static str {
    indoc! {r#"
        workspace {
          model {
            system = softwareSystem "System" {
              app = container "App"
            }

            live = deploymentEnvironment "Live" {
              node = deploymentNode "Node" {
                instance = containerInstance app {
                  healthCheck "Ping" https://example.com/health
                }
              }
            }
          }

          views {
            systemContext system "system-context" "System context" {
              include *
              title Overview
              autoLayout lr 300 400
            }

            filtered "system-context" include "TeamA" "filtered-view" {
              default
            }

            custom "custom-view" "Custom Title" {
              title "Dashboard"
            }

            styles {
              theme https://example.com/theme1
              themes https://example.com/theme2 https://example.com/theme3

              element "TeamA" {
                shape RoundedBox
                border Dashed
                iconPosition Top
                style Dotted
                routing Orthogonal
                metadata false
              }

              relationship "Async" {
                color #ffffff
              }
            }

            branding {
              logo logo.png
              font "Example" https://example.com/font
            }

            terminology {
              enterprise "Enterprise"
            }
          }

          configuration {
            scope softwareSystem
            visibility public
            users {
              alice admin
            }
          }
        }
    "#}
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
                 promotedSystem = instanceOf system blue "Promoted"
                 canarySystem = softwareSystemInstance system blue "Canary"
                 canaryApp = containerInstance app blue "Canary"
               }
            }
          }
        }
    "#};

    let captures = highlight_captures(source);

    for keyword in ["softwareSystemInstance", "instanceOf", "containerInstance"] {
        assert_has_capture(&captures, "keyword", keyword);
    }

    for ty in [
        "systemInstance",
        "promotedSystem",
        "canarySystem",
        "canaryApp",
        "system",
        "app",
    ] {
        assert_has_capture(&captures, "type", ty);
    }

    let blue_type_count = captures
        .iter()
        .filter(|(name, text)| name == "type" && text == "blue")
        .count();
    assert_eq!(
        blue_type_count, 4,
        "expected `blue` to be highlighted as a type in its declaration and all three instance references, got {captures:#?}"
    );
}

#[test]
fn directives_highlight_preproc_paths_importers_constants_and_booleans() {
    let source = indoc! {r#"
        workspace {
          !const "NAME" "Name"
          !constant ENV prod
          !var DESCRIPTION docs
          !identifiers hierarchical
          !impliedRelationships false
          !docs docs com.example.documentation.CustomDocumentationImporter
          !adrs decisions madr

          model {
            system = softwareSystem "System"

            !relationships "*->*" {
              tag "Async"
            }

            !element system {
              url https://example.com/element
            }
          }
        }
    "#};

    let captures = highlight_captures(source);

    assert_all_have_capture(
        &captures,
        "preproc",
        &[
            "!const",
            "!constant",
            "!var",
            "!identifiers",
            "!impliedRelationships",
            "!docs",
            "!adrs",
            "!relationships",
            "!element",
        ],
    );

    assert_all_have_capture(&captures, "constant", &["\"NAME\"", "ENV"]);
    assert_all_have_capture(
        &captures,
        "string.special",
        &["docs", "decisions", "\"*->*\""],
    );

    assert_has_capture(&captures, "variable", "DESCRIPTION");
    assert_has_capture(&captures, "constant.builtin", "hierarchical");
    assert_has_capture(&captures, "boolean", "false");
    assert_has_capture(
        &captures,
        "type",
        "com.example.documentation.CustomDocumentationImporter",
    );
    assert_has_capture(&captures, "type.builtin", "madr");
    assert_has_capture(&captures, "tag", "\"Async\"");
    assert_has_capture(
        &captures,
        "string.special.url",
        "https://example.com/element",
    );
}

#[test]
fn styles_auxiliary_blocks_and_configuration_highlight_builtins_tags_titles_and_urls() {
    let captures = highlight_captures(styles_highlight_fixture());

    assert_all_have_capture(
        &captures,
        "keyword",
        &[
            "systemContext",
            "filtered",
            "custom",
            "theme",
            "themes",
            "branding",
            "logo",
            "font",
            "healthCheck",
            "terminology",
            "scope",
            "visibility",
            "users",
        ],
    );

    assert_all_have_capture(
        &captures,
        "constant.builtin",
        &[
            "include",
            "lr",
            "Dashed",
            "Top",
            "Dotted",
            "Orthogonal",
            "softwareSystem",
            "public",
        ],
    );

    assert_all_have_capture(
        &captures,
        "title",
        &["Overview", "\"Custom Title\"", "\"Dashboard\""],
    );
    assert_all_have_capture(&captures, "tag", &["\"TeamA\"", "\"Async\""]);
    assert_all_have_capture(
        &captures,
        "string.special",
        &[
            "https://example.com/theme1",
            "https://example.com/theme2",
            "https://example.com/theme3",
            "logo.png",
            "RoundedBox",
        ],
    );

    assert_has_capture(&captures, "string.special.url", "https://example.com/font");
    assert_has_capture(
        &captures,
        "string.special.url",
        "https://example.com/health",
    );
    assert_has_capture(&captures, "boolean", "false");
    assert_has_capture(&captures, "type.builtin", "enterprise");
    assert_has_capture(&captures, "string", "alice");
    assert_has_capture(&captures, "string", "admin");
}
