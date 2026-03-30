workspace "Some System" "Description" {
    !docs docs com.example.documentation.CustomDocumentationImporter
    !adrs decisions madr

    model {
        contributor = person "Person"
        someSystem = softwareSystem "Some System" {
            !docs ../docs com.example.documentation.NestedDocumentationImporter
            !adrs ../decisions com.example.documentation.CustomDecisionImporter

            someContainer = container "Some Container" "" ""
        }
    }
}
