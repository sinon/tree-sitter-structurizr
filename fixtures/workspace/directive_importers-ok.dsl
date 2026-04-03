workspace "Some System" "Description" {
    !docs docs
    !adrs decisions madr

    model {
        contributor = person "Person"
        someSystem = softwareSystem "Some System" {
            !docs docs
            !adrs decisions adrtools

            someContainer = container "Some Container" "" ""
        }
    }
}
