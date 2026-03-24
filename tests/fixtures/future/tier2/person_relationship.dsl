workspace {
    model {
        user = person "User"
        softwareSystem = softwareSystem "System"

        user -> softwareSystem "Uses"
    }
}
