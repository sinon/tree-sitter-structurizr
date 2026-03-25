workspace {
    model {
        system = softwareSystem "System" {
            api = container "API" "Handles requests" "Rust" {
                worker = component "Worker" "Processes jobs" "Rust"
            }
        }
    }
}
