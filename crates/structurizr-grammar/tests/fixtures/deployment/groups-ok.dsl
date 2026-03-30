workspace {
    !identifiers hierarchical

    model {
        softwareSystem = softwareSystem "Software System" {
            database = container "DB"
            api = container "API" {
                -> database "Uses"
            }
        }

        deploymentEnvironment "WithDeploymentGroups" {
            serviceInstance1 = deploymentGroup "Service Instance 1"
            serviceInstance2 = deploymentGroup "Service Instance 2"

            deploymentNode "Server 1" {
                containerInstance softwareSystem.api serviceInstance1
                containerInstance softwareSystem.database serviceInstance1
            }

            deploymentNode "Server 2" {
                containerInstance softwareSystem.api serviceInstance2
                containerInstance softwareSystem.database serviceInstance2
            }
        }
    }
}
