workspace {
    model {
        user = person "User"
        system = softwareSystem "System" {
            api = container "API" {
                worker = component "Worker"
            }
        }
        platform = softwareSystem "Platform"
    }
}
