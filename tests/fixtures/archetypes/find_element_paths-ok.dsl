workspace extends amazon-web-services.dsl {
    model {
        !element "DeploymentNode://Live/Amazon Web Services" {
            deploymentNode "New deployment node" {
                infrastructureNode "New infrastructure node" {
                    -> route53
                }
            }
        }

        !element region {
            deploymentNode "New deployment node 2" {
                infrastructureNode "New infrastructure node 2" {
                    -> route53
                }
            }
        }
    }

    views {
        deployment * "Live" {
            include *
            autoLayout lr
        }
    }
}
