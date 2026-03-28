workspace {
    model {
        user = person "User"
        system = softwareSystem "System" {
            api = container "API" {
                worker = component "Worker"
            }
        }

        user -> system "Uses"
        user -> api "Calls"
        user -> worker "Triggers"
    }

    views {
        systemContext system "system-context" {
            include user
        }

        container system "container-view" {
            include api
        }

        component api "component-view" {
            include worker
        }
    }
}
