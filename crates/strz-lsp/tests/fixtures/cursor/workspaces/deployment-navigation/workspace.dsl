workspace {
    model {
        system = softwareSystem "System" {
            <CURSOR:api-declaration>api = container "API"
        }

        live = deploymentEnvironment "Live" {
            <CURSOR:primary-deployment-node>primary = deploymentNode "Primary" {
                <CURSOR:gateway-declaration>gateway = infrastructureNode "Gateway"
                <CURSOR:api-instance-declaration>apiInstance = containerInstance <CURSOR:api-target>api {
                    <CURSOR:gateway-relationship>gateway -> <CURSOR:deferred-this>this "Routes traffic"
                }
                softwareSystemInstance <CURSOR:system-target>system
            }
            secondary = deploymentNode "Secondary" {
                secondaryApiInstance = containerInstance api
            }

            <CURSOR:primary-relationship>primary -> secondary "Replicates traffic"
            gateway -> <CURSOR:secondary-api-instance-relationship>secondaryApiInstance "Routes traffic"
        }
    }
}
