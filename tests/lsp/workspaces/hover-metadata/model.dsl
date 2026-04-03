model {
    system = softwareSystem "Payments Platform" {
        api = container "Payments API" "Processes payment requests" "Rust" "Internal, HTTP" {
            technology "Axum"
            tags "Internal, Edge"
            url "https://example.com/api"
        }
        worker = container "Settlement Worker" "Settles payment jobs" "Rust"
    }

    rel = api -> worker "Publishes jobs" "NATS" "Async, Messaging" {
        description "Delivers asynchronous jobs"
        tag "Observed"
        url "https://example.com/rel"
    }
}
