workspace {

    model {
        ss = softwaresystem "Software System" {
            webapp = container "Web Application" {
                tag "UI"
            }
            db = container "Database Schema" {
                tag "DB"
            }
        }

        webapp -> db

        live = deploymentEnvironment "Live" {
            dn = deploymentNode "Deployment Node" {
                webappInstance = containerInstance webapp
                dbInstance = containerInstance db
            }
        }
    }

    views {
        deployment ss "Live" {
            include *

            animation {
                webappInstance
                dbInstance
            }
        }

        deployment ss "Live" {
            include *

            animation {
                webapp
                db
            }
        }

        deployment ss "Live" {
            include *

            animation {
                webapp
                webapp->
            }
        }

        deployment ss "Live" {
            include *

            animation {
                webappInstance
                webappInstance->
            }
        }

        deployment ss "Live" {
            include *

            animation {
                element.tag==UI
                element.tag==DB
            }
        }
    }

}
