workspace {
    !identifiers hierarchical
    !impliedRelationships false
    !docs "docs"
    !adrs "docs/adrs"

    model {
        user = person "User"
        system = softwareSystem "System" {
            app = container "App"
        }

        user -> system.app "Uses" "HTTPS"

        live = deploymentEnvironment "Live" {
            node = deploymentNode "Node" {
                systemInstance = softwareSystemInstance system
            }
        }
    }

    views {
        properties {
            "plantuml.url" "https://plantuml.com/plantuml"
        }

        dynamic system "dynamic-view" {
            1: user -> system.app "Requests data" "HTTPS"
            autoLayout lr
            title "Dynamic"
        }

        deployment * "Live" "deployment-view" {
            include *
            autoLayout
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
