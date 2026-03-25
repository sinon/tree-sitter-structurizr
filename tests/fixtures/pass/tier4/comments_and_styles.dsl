/*
 * This is a combined version of the following workspaces:
 *
 * - "Big Bank plc - System Landscape"
 * - "Big Bank plc - Internet Banking System"
 */
workspace {
    views {
        systemContext financialRiskSystem "Context" "An example System Context diagram for the Financial Risk System architecture kata." {
            include *
            autoLayout
        }

        styles {
            element "Software System" {
                background #801515
                shape RoundedBox
                color #ffffff
                opacity 30
            }

            element "Person" {
                background #d46a6a
                shape Person
                color white
            }

            relationship "Future State" {
                opacity 30
            }
        }
    }
}
