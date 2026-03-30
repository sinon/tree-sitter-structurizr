workspace {

    !const "NAME" "Name"
    !constant NAME VALUE
    !var "DESCRIPTION" "Description"

    model {
        !var SOFTWARE_SYSTEM_NAME "Software System 1"
        !include include/model/software-system/model.dsl

        !const FOLDER include/model
        !include include/model/software-system
        !include https://raw.githubusercontent.com/structurizr/java/refs/heads/master/structurizr-dsl/src/test/resources/dsl/include/model.dsl
    }

}
