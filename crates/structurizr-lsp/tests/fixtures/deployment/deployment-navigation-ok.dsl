workspace {
    model {
        system = softwareSystem "System" {
            api = container "API"
        }

        live = deploymentEnvironment "Live" {
            primary = deploymentNode "Primary" {
                gateway = infrastructureNode "Gateway"
                apiInstance = containerInstance api {
                    gateway -> this "Routes traffic"
                }
                softwareSystemInstance system
            }
        }
    }
}
