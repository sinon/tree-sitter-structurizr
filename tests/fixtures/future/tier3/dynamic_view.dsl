workspace {
    model {
        user = person "User"
        system = softwareSystem "System"

        user -> system "Uses"
    }

    views {
        dynamic system "dynamic-view" {
            user -> system "Requests data"
        }
    }
}
