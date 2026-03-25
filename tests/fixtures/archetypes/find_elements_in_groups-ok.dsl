workspace {
    model {
        properties {
            "structurizr.groupSeparator" "/"
        }

        user1 = person "User 1"
        user2 = person "User 2"

        department1 = group "Department 1" {
            team1 = group "Team 1" {
                softwareSystem "A"
            }

            team2 = group "Team 2" {
                softwareSystem "B"
            }
        }

        !elements department1 {
            user1 -> this "Uses"
        }

        !elements team1 {
            user2 -> this "Uses"
        }
    }
}
