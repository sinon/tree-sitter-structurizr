workspace{
views{
properties{
"plantuml.url" "https://plantuml.com/plantuml"
}
!const SOURCE """
class MyClass
"""
image * "image"{
plantuml """
 @startuml

 ${SOURCE}
 @enduml
"""
}
}
}
