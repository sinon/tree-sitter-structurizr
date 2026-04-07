workspace {
    model {
        system = softwareSystem "System" {
            app = container "App"
        }

        live = deploymentEnvironment "Live" {
            blue = deploymentGroup "Blue"

            node = deploymentNode "Node" {
                systemInstance = softwareSystemInstance system
                canarySystem = softwareSystemInstance system blue "Canary"
                canaryApp = containerInstance app blue "Canary"
            }
        }
    }
}
