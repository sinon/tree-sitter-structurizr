workspace {
    model {
        system = softwareSystem "System"
    }

    views {
        systemlandscape "landscape" {
            include *
            autoLayout
        }

        systemcontext system "context" {
            include *
            animation {
                system
            }
            autoLayout
        }
    }
}
