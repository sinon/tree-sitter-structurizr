workspace {
    model {
        user = person "User"
        system = softwareSystem "System"

        rel = user -> "Uses"
    }

    views {
        systemLandscape {
            include rel
        }
    }
}
