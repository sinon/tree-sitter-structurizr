workspace {
    model {
        user = person "User"
        sysa = softwareSystem "Software System A" "A description of software system A."
        sysb = softwareSystem "Software System B" "A description of software system B." Future

        user -> sysa "Uses for tasks 1 and 2" "" Current
        user -> sysb "Uses for task 2" "" Future
    }

    views {
        systemLandscape FullLandscape {
            include *
        }

        filtered FullLandscape exclude Future CurrentLandscape "The current system landscape."
        filtered FullLandscape exclude Current FutureLandscape "The future state system landscape after Software System B is live."
    }
}
