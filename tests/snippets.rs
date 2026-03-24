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
fn parses_model_elements_and_relationships_without_errors() {
    let source = indoc! {r#"
        workspace {
            model {
                user = person "User"
                system = softwareSystem "System"

                user -> system "Uses"
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::model_elements", &tree, source);
}

#[test]
fn parses_nested_containers_and_components_without_errors() {
    let source = indoc! {r#"
        workspace {
            model {
                system = softwareSystem "System" {
                    api = container "API" "Handles requests" "Rust" {
                        worker = component "Worker" "Processes jobs" "Rust"
                    }
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::nested_elements", &tree, source);
}

#[test]
fn tracks_extended_model_features_as_pending_future_coverage() {
    let source = indoc! {r#"
        workspace {
            model {
                user = person "User"
                group "Internal" {
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_has_errors("inline::model_group_pending", &tree, source);
}

#[test]
fn parses_core_views_without_errors() {
    let source = indoc! {r#"
        workspace {
            model {
                system = softwareSystem "System" {
                    api = container "API"
                }
            }

            views {
                systemLandscape "landscape" "Overview" {
                    include *
                    autoLayout lr 300 200
                    title "Landscape"
                }

                systemContext system "system-context" "System context" {
                    include *
                    exclude api
                    description "System context"
                }

                container system "container-view" {
                    include *
                    default
                }

                component api "component-view" {
                    include *
                    title "Components"
                }

                filtered "container-view" include "Element,Relationship" "filtered-view" {
                    default
                    title "Filtered"
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::core_views", &tree, source);
}

#[test]
fn tracks_dynamic_views_as_pending_future_coverage() {
    let source = indoc! {r#"
        workspace {
            model {
                user = person "User"
                system = softwareSystem "System"

                user -> system "Uses"
            }

            views {
                dynamic system "dynamic-view" {
                    user -> system "Requests data"
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_has_errors("inline::dynamic_view_pending", &tree, source);
}
