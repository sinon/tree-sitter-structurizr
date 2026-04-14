workspace {
    !include "alpha.dsl"
    !include "beta.dsl"

    model {
        user = person "User"
        user -> <CURSOR>api "Calls"
    }
}
