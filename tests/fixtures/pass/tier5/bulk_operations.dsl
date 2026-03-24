workspace {

    model {
        user = person "User"

        !identifiers flat

        softwareSystem1 = softwareSystem "Software System 1" {
            application1 = container "Application" {
                component "ComponentA"
                component "ComponentB"
                component "ComponentC"
            }

            databaseSchema1 = container "Database Schema"

            !elements "element.parent==application1" {
                tags "Tag 1"
                user -> this "Uses 1"
                this -> databaseSchema1 "Uses 1" {
                    tags "Tag"
                }
            }

            !elements "element.parent==application1" {
                -> databaseSchema1 "Uses 2" {
                    tags "Tag"
                }
            }
        }
    }

}
