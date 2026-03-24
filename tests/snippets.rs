mod common;

use indoc::indoc;

#[test]
fn parses_placeholder_hello_snippet_without_errors() {
    let source = "hello";
    let tree = common::parse(source);

    common::assert_no_errors("inline::hello", &tree, source);
}

#[test]
fn tracks_workspace_scaffold_as_pending_grammar_coverage() {
    let source = indoc! {r#"
        workspace {
            model {
            }

            views {
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_has_errors("inline::workspace", &tree, source);
}
