(comment) @comment

(string) @string

(hex_color) @string.special

(named_color) @string.special

(number) @number

(order) @number

(wildcard) @constant.builtin

(layout_direction) @constant.builtin

(this_keyword) @variable.special

(relationship_operator) @operator

(default_statement) @keyword

[
  "!include"
  "!identifiers"
  "!impliedRelationships"
  "!docs"
  "!adrs"
  "!elements"
  "!element"
] @preproc

[
  "workspace"
  "extends"
  "model"
  "views"
  "configuration"
  "archetypes"
  "person"
  "softwareSystem"
  "softwaresystem"
  "container"
  "component"
  "deploymentEnvironment"
  "deploymentGroup"
  "deploymentNode"
  "infrastructureNode"
  "containerInstance"
  "softwareSystemInstance"
  "description"
  "technology"
  "tag"
  "tags"
  "metadata"
  "name"
  "title"
  "include"
  "exclude"
  "autoLayout"
  "autolayout"
  "animation"
  "systemLandscape"
  "systemContext"
  "filtered"
  "dynamic"
  "deployment"
  "custom"
  "styles"
  "light"
  "dark"
  "element"
  "relationship"
  "properties"
  "theme"
  "themes"
  "plantuml"
  "mermaid"
  "kroki"
  "image"
  "scope"
  "visibility"
  "users"
] @keyword

[
  "{"
  "}"
] @punctuation.bracket

":" @punctuation.delimiter

"=" @operator

"->" @operator

(person
  identifier: (identifier) @variable)

(software_system
  identifier: (identifier) @type)

(container
  identifier: (identifier) @type)

(component
  identifier: (identifier) @type)

(custom_element
  identifier: (identifier) @type)

(archetype_definition
  identifier: (identifier) @type)

(archetype_instance
  kind: (identifier) @type)

(deployment_environment
  identifier: (identifier) @type)

(deployment_group
  identifier: (identifier) @type)

(deployment_node
  identifier: (identifier) @type)

(infrastructure_node
  identifier: (identifier) @type)

(relationship
  source: (identifier) @variable
  destination: (identifier) @variable)

(dynamic_relationship
  source: (identifier) @variable
  destination: (identifier) @variable)

(style_setting
  name: (identifier) @property)

(property_entry
  name: [
    (identifier)
    (bare_value)
    (string)
  ] @property)
