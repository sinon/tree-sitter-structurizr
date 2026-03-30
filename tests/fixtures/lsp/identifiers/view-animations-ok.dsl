workspace {
    model {
        user = person "User"
        system = softwareSystem "System" {
            web = container "Web"
            api = container "API" {
                worker = component "Worker"
            }
        }

        live = deploymentEnvironment "Live" {
            primary = deploymentNode "Primary" {
                webInstance = containerInstance web
            }
            secondary = deploymentNode "Secondary" {
                apiInstance = containerInstance api
            }
        }
    }

    views {
        systemContext system "system-context" {
            include user system
            animation {
                user system
            }
        }

        container system "container-view" {
            include api worker
            animation {
                api worker
            }
        }

        deployment system "Live" {
            include primary secondary webInstance apiInstance
            animation {
                primary secondary
                webInstance apiInstance
            }
        }
    }
}
