workspace {
  model {
    1user = person "User"
    system1 = softwareSystem "System" {
      1api = container "API"
    }

    1user -> 1api "Uses"
  }
}
