workspace {
    model {
        custom = element "Element"

        s = softwareSystem "Software System" {
            c = container "Container"
        }

        live = deploymentEnvironment "live" {
            deploymentNode "Live" {
                in = infrastructureNode "Infrastructure Node" {
                    custom -> this
                }

                dn = deploymentNode "Deployment Node" {
                    in -> this

                    softwareSystemInstance s {
                        in -> this
                    }

                    containerInstance c {
                        in -> this
                    }
                }
            }
        }
    }
}
