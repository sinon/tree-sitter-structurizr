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
                group "Internal" {
                    properties {
                        "mode" "pending"
                    }
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_has_errors("inline::model_group_properties_pending", &tree, source);
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
fn parses_advanced_views_directives_and_configuration_without_errors() {
    let source = indoc! {r#"
        workspace {
            !identifiers hierarchical
            !impliedRelationships false
            !docs "docs"
            !adrs "docs/adrs"

            model {
                user = person "User"
                system = softwareSystem "System"

                user -> system "Uses"
            }

            views {
                dynamic system "dynamic-view" {
                    1: user -> system "Requests data" "HTTPS"
                    autoLayout lr
                    title "Dynamic"
                }

                deployment * "Live" "deployment-view" {
                    include *
                    autoLayout
                }

                custom "custom-view" "Custom title" {
                    include user system
                    description "Custom description"
                }

                image * "image-view" {
                    plantuml "diagram.puml"
                    title "Architecture image"
                }
            }

            configuration {
                scope landscape
                visibility private

                users {
                    "alice@example.com" read
                    "bob@example.com" write
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::advanced_tier4", &tree, source);
}

#[test]
fn parses_block_comments_and_view_styles_without_errors() {
    let source = indoc! {r#"
        /*
         * This is a combined version of the following workspaces:
         *
         * - "Big Bank plc - System Landscape"
         * - "Big Bank plc - Internet Banking System"
         */
        workspace {
            views {
                systemContext financialRiskSystem "Context" "An example System Context diagram for the Financial Risk System architecture kata." {
                    include *
                    autoLayout
                }

                styles {
                    element "Software System" {
                        background #801515
                        shape RoundedBox
                        color #ffffff
                        opacity 30
                    }

                    element "Person" {
                        background #d46a6a
                        shape Person
                        color white
                    }

                    relationship "Future State" {
                        opacity 30
                    }
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::block_comments_and_styles", &tree, source);
}

#[test]
fn parses_archetypes_and_custom_elements_without_errors() {
    let source = indoc! {r#"
        workspace {
            model {
                archetypes {
                    hardwareSystem = element {
                        metadata "Hardware System"
                        tag "Hardware System"
                    }

                    application = container {
                        tag "Application"
                    }

                    https = -> {
                        technology "HTTPS"
                    }
                }

                a = softwareSystem "A"
                b = hardwareSystem "B"

                application1 = application "Application 1" {
                    a --https-> this "Uses"
                }

                a -> b "Gets data from"
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::archetypes_and_custom_elements", &tree, source);
}

#[test]
fn parses_bulk_element_operations_without_errors() {
    let source = indoc! {r#"
        workspace {
            model {
                user = person "User"

                !identifiers flat

                softwareSystem1 = softwareSystem "Software System 1" {
                    application1 = container "Application" {
                        component "ComponentA"
                        component "ComponentB"
                    }

                    databaseSchema1 = container "Database Schema"

                    !elements "element.parent==application1" {
                        tags "Tag 1"
                        user -> this "Uses 1"
                        this -> databaseSchema1 "Uses 1" {
                            tags "Tag"
                        }
                    }
                }
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::bulk_element_operations", &tree, source);
}

#[test]
fn parses_deployment_environments_and_views_without_errors() {
    let source = indoc! {r#"
        workspace {
            !identifiers hierarchical

            model {
                softwareSystem = softwareSystem "Software System" {
                    api = container "API"
                    database = container "DB"
                }

                live = deploymentEnvironment "Live" {
                    blue = deploymentGroup "Blue"

                    aws = deploymentNode "Amazon Web Services" "" "" "Amazon Web Services - Cloud" {
                        region = deploymentNode "us-east-1" {
                            elb = infrastructureNode "Elastic Load Balancer"

                            server = deploymentNode "Server" {
                                apiInstance = containerInstance softwareSystem.api blue
                                databaseInstance = containerInstance softwareSystem.database blue
                            }
                        }
                    }

                    elb -> apiInstance "Routes to"
                }
            }

            views {
                deployment softwareSystem "Live" "live-deployment" {
                    include *
                    autolayout lr

                    animation {
                        live.aws.region.elb
                        live.aws.region.server.apiInstance
                        live.aws.region.server.databaseInstance
                    }
                }

                themes https://example.com/theme.json
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_no_errors("inline::deployment_model_and_view", &tree, source);
}

#[test]
fn tracks_script_blocks_as_pending_future_coverage() {
    let source = indoc! {r#"
        workspace {
            !script groovy {
                println "hello"
            }

            views {
            }
        }
    "#};
    let tree = common::parse(source);

    common::assert_has_errors("inline::script_pending", &tree, source);
}
