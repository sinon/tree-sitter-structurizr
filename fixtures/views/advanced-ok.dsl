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
