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
