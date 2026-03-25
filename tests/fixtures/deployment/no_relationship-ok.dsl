workspace {
    !identifiers hierarchical

    model {
        ss = softwareSystem "Software System" {
            ui = container "UI" "Description" "JavaScript and React"
            backend = container "Backend" "Description" "Spring Boot"

            ui -> backend "Makes API requests to" "JSON/HTTPS"
        }

        one = deploymentEnvironment "One" {
            deploymentNode "Developer's Computer" {
                deploymentNode "Web Browser" {
                    instanceOf ss.ui
                }
                instanceOf ss.backend
            }
        }

        three = deploymentEnvironment "Three" {
            computer = deploymentNode "User's Computer" {
                webbrowser = deploymentNode "Web Browser" {
                    ui = instanceOf ss.ui
                }
            }
            datacenter = deploymentNode "Data Center" {
                loadbalancer = infrastructureNode "Load Balancer"
                server = deploymentNode "Server" {
                    backend = instanceOf ss.backend
                }
            }

            computer.webbrowser.ui -/> datacenter.server.backend {
                computer.webbrowser.ui -> datacenter.loadbalancer
                datacenter.loadbalancer -> datacenter.server.backend "Forwards API requests to" ""
            }
        }
    }
}
