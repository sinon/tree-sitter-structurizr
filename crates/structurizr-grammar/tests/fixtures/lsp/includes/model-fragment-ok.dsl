model {
    user = person "User"
    system = softwareSystem "System" {
        api = container "API"
    }

    user -> api "Uses"
}
