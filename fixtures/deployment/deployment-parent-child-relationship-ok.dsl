workspace {
    model {
        system = softwareSystem "System" {
            api = container "API"
        }

        live = deploymentEnvironment "Live" {
            primary = deploymentNode "Primary" {
                gateway = infrastructureNode "Gateway"
                apiInstance = containerInstance api
            }

            primary -> gateway "Hosts traffic"
        }
    }
}
