workspace {
    model {
        archetypes {
            service = element {
                tag SharedQuality
                tags "Customer Facing,Reusable Pattern"
                tags "Async Messaging" "Operational Data"
            }
        }

        customer = person "Customer" "Uses the platform" "Customer Facing,SharedQuality" {
            tag "External User"
            tags "People,Journey Mapping"
            tags "Human Actor" "Primary Persona"
        }

        platform = softwareSystem "Payments Platform" "Processes payments" "Internal Only,SharedQuality" {
            tag "Core System"
            tags "Operational Data,Important Capability"
            tags "Domain Logic" "Revenue Flow"

            api = container "Public API" "Handles requests" "Rust" "Async Messaging,SharedQuality" {
                tag "API Surface"
                tags "Integration,Edge Routing"
                tags "HTTP Boundary" "Public Contract"

                worker = component "Worker" "Processes jobs" "Rust" "Operational Data,SharedQuality" {
                    tag "Background Processing"
                    tags "Queues,Async Messaging"
                    tags "Retry Policy" "Batch Work"
                }
            }
        }

        ledger = element "Ledger" "Domain Entity" "Stores journal entries" "Operational Data,SharedQuality" {
            tag "Accounting"
            tags "Reporting,Compliance"
            tags "Financial Record" "Reference Data"
        }

        reporting = service "Reporting Service" "Analytics" "Supports finance reporting" "Customer Facing,SharedQuality" {
            tag "Derived Service"
            tags "Operational Data,Reusable Pattern"
            tags "Batch Analytics" "Scheduled Work"
        }

        !element platform {
            tag "Extended Platform"
            tags "Platform Team,SharedQuality"
            tags "Directive Tag" "Directive Batch"
        }

        rel = customer -> api "Uses API" "HTTPS" "Customer Facing,SharedQuality" {
            tag "Critical Path"
            tags "Request Flow,Sync Call"
            tags "Primary Interaction" "Observed Traffic"
        }

        futureRel = customer -> platform "Tracks future capabilities" "" Future

        !elements "element.tag==SharedQuality" {
            tag "Bulk Element Tag"
            tags "Bulk Element,SharedQuality"
            tags "Bulk Batch" "Bulk Group"
        }

        !relationships "*->*" {
            tag "Bulk Relationship Tag"
            tags "Async Messaging,SharedQuality"
            tags "Bulk Routing" "Bulk Transport"
        }

        live = deploymentEnvironment "Live" {
            blue = deploymentGroup "Blue"

            edge = deploymentNode "Edge" "Public ingress" "Kubernetes" 2 "Edge Routing,SharedQuality" {
                tag "Deployment Node Tag"
                tags "Operations,Platform Runtime"
                tags "Blue Cluster" "Ingress Tier"

                gateway = infrastructureNode "Gateway" "Routes traffic" "HAProxy" "Edge Routing,SharedQuality" {
                    tag "Infrastructure Tag"
                    tags "Network,SharedQuality"
                    tags "North South" "Traffic Control"
                }

                platformCanary = softwareSystemInstance platform blue "Customer Facing,SharedQuality" {
                    tag "Canary Release"
                    tags "Operations,Observed"
                    tags "Gradual Rollout" "Instance Metrics"
                }

                apiReplica = containerInstance api blue "Async Messaging,SharedQuality" {
                    tag "Replica"
                    tags "Operations,Observed"
                    tags "Queue Consumer" "Runtime Thread"
                }

                workerReplica = instanceOf api blue "Edge Routing,SharedQuality" {
                    tag "Alias Instance"
                    tags "Operations,Observed"
                    tags "Directive Replay" "Compatibility Check"
                }
            }
        }
    }

    views {
        systemContext platform "platform-context" {
            include *
        }

        filtered "platform-context" include "SharedQuality,Customer Facing" "quality-view" {
            title "Quality tags"
        }

        filtered "platform-context" exclude Future "future-view" {
            title "Future tags"
        }

        styles {
            element SharedQuality {
                background #1168bd
            }

            relationship Future {
                color #d46a00
            }
        }
    }
}
