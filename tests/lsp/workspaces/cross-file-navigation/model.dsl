model {
    user = person "User"
    system = softwareSystem "System"
    rel = user -> system "Uses"
}
