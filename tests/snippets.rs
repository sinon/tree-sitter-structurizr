mod common;

use indoc::indoc;

#[test]
fn parses_minimal_workspace_without_errors() {
    let source = indoc! {r#"
        workspace {
            model {
            }

            views {
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::minimal_workspace", &tree, source);
}

#[test]
fn parses_workspace_metadata_and_comments_without_errors() {
    let source = indoc! {r#"
        # leading comment
        workspace "Payments" "Core architecture" {
            name "Payments"
            description "Core architecture"

            model {
            }

            views {
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::workspace_metadata", &tree, source);
}

#[test]
fn tracks_model_contents_as_pending_tier_two_coverage() {
    let source = indoc! {r#"
        workspace {
            model {
                user = person "User"
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_has_errors("inline::model_contents_pending", &tree, source);
}
