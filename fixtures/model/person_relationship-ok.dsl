workspace {
    model {
        user = person "User"
        system = softwareSystem "System"

        user -> system "Uses"
    }
}
