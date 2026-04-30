model {
    system = softwareSystem "System" {
        api = container "API" {
            worker = component "Worker"
        }
    }
}
