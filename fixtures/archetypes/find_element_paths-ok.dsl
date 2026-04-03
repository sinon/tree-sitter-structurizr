workspace extends "../deployment/aws-ok.dsl" {
    model {
        !element "DeploymentNode://Live/Amazon Web Services" {
            deploymentNode "New deployment node" {
                infrastructureNode "New infrastructure node" {
                    -> live.aws.region.route53
                }
            }
        }

        !element live.aws.region {
            deploymentNode "New deployment node 2" {
                infrastructureNode "New infrastructure node 2" {
                    -> live.aws.region.route53
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
