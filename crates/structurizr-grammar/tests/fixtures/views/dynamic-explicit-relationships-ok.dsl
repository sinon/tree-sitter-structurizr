workspace {

    model {
        a = softwareSystem "A"
        b = softwareSystem "B"

        r1 = a -> b "Sync" {
            tags "Sync"
        }

        r2 = a -> b "Async" {
            tags "Async"
        }
    }

    views {
        dynamic * {
            r2 "Async"
            autoLayout
        }
    }

}
