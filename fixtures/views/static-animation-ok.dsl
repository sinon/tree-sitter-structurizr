workspace {

    model {
        a = softwareSystem "A"
        b = softwareSystem "B"

        a -> b
    }

    views {
        systemLandscape {
            include *

            animation {
                a
                b
            }
        }

        systemLandscape {
            include *

            animation {
                a
                a->
            }
        }
    }

}
