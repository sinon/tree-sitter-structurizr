workspace {
    !identifiers hierarchical

    model {
        !include "model.dsl"

        user = person "User"
        user -> system "Uses"
        user -> api "Calls"
    }
}
