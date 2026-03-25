workspace {
    model {
        system = softwareSystem "System" {
            app = container "App"
        }

        deploymentEnvironment "Live" {
            deploymentNode "Failover" "" "" "Failover" {
                standby = containerInstance app "Failover"
            }
        }
    }
}
