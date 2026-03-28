workspace {
    model {
        a = softwareSystem "A"
        b = softwareSystem "B"

        rel = a -> b "Async"
    }

    views {
        dynamic * {
            rel "Async"
        }
    }
}
