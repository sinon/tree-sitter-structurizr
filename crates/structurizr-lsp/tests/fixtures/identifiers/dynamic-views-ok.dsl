workspace {
    model {
        system = softwareSystem "System" {
            web = container "Web Application"
            api = container "API Application" {
                signin = component "Sign In Controller"
                security = component "Security Component"
            }
            database = container "Database"
        }
    }

    views {
        dynamic api "SignIn" {
            web -> signin "Submits credentials to"
            signin -> security "Validates credentials using"
            security -> database "Reads users from"
        }
    }
}
