workspace {
    !identifiers flat

    model {
        !identifiers hierarchical

        system = softwareSystem "System" {
            api = container "API" {
                worker = component "Worker"
            }
        }

        !element system.api.worker {
            properties {
                "team" "Core"
            }
        }
    }
}
