workspace {
    model {
        !impliedRelationships false

        enterprise "Organisation" {
            system = softwareSystem "System" {
                app = container "App" {
                    controller = component "Controller" {
                        url "https://example.com/controller"
                        properties {
                            "tier" "web"
                        }
                        perspectives {
                            "Security" "Reviewed"
                            perspective "Ownership" {
                                value "Team A"
                                description "Owned by Team A"
                            }
                        }
                    }
                }
            }
        }

        user = person "User" {
            url "https://example.com/user"
            perspectives {
                "Technical Debt" "Feature pressure" "High"
            }
        }

        user -> controller "Uses"

        !relationships "*->*" {
            tag "Async"
            url "https://example.com/relationships"
            properties {
                "latency" "medium"
            }
            perspectives {
                "Latency" "Monitor closely"
            }
        }

        live = deploymentEnvironment "Live" {
            deploymentNode "Node" {
                infrastructureNode "Load Balancer" {
                    url "https://example.com/lb"
                    perspectives {
                        "Operations" "Monitored"
                    }
                }

                appInstance = containerInstance app {
                    url "https://example.com/app"
                    properties {
                        "tier" "runtime"
                    }
                    perspectives {
                        "Availability" "Measured"
                    }
                    healthCheck "Ping" "https://example.com/health"
                    healthCheck "Ready" "https://example.com/ready" 60
                    healthCheck "Live" "https://example.com/live" 120 1000
                }

                replica = instanceOf app {
                    url "https://example.com/replica"
                }
            }
        }
    }

    views {
        systemContext system "system-context" "System context" {
            include *
            properties {
                "view" "context"
            }
            default
        }

        dynamic app "dynamic-view" "Dynamic view" {
            user -> controller "Requests"
            controller -> user {
                url "https://example.com/reply"
            }
            properties {
                "view" "dynamic"
            }
            default
        }

        styles {
            theme https://example.com/theme1
            themes https://example.com/theme2 https://example.com/theme3
        }

        branding {
            logo logo.png
            font "Example" https://example.com/font
        }

        terminology {
            enterprise "Enterprise"
            person "Person"
            softwareSystem "Software System"
            container "Container"
            component "Component"
            deploymentNode "Deployment Node"
            infrastructureNode "Infrastructure Node"
            relationship "Relationship"
        }
    }

    configuration {
        users {
            user@example.com read
        }
    }
}
