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
            include *
            animation {
                primary
                webInstance apiInstance
            }
        }
    }
}
